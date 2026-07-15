use rand::RngCore;
use rustls::pki_types::ServerName;
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};
use serde::Deserialize;
use sha3::{Digest, Sha3_256};
use std::env;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant};

const WALLET_HEX_LEN: usize = 3904;
const BATCH_PER_WORKER: u64 = 1_000_000;
const CUDA_BATCH_NONCES: u64 = 1_024_000_000;
const STATUS_INTERVAL: Duration = Duration::from_secs(2);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

#[cfg(feature = "cuda")]
extern "C" {
    fn ql_cuda_mine(
        previous_hash: *const u8,
        previous_hash_len: usize,
        merkle_root: *const u8,
        merkle_root_len: usize,
        block_height: u64,
        wallet: *const u8,
        wallet_len: usize,
        difficulty_bits: u32,
        start_nonce: u64,
        total_nonces: u64,
        found_nonce: *mut u64,
        checked: *mut u64,
        device_id: i32,
    ) -> i32;
}

#[derive(Clone)]
struct Template {
    block_height: u64,
    previous_block_hash: Vec<u8>,
    merkle_root: Vec<u8>,
    difficulty_bits: u32,
}

#[derive(Debug, Deserialize)]
struct TemplateResponse {
    block_height: u64,
    previous_block_hash: String,
    merkle_root: String,
    #[serde(default = "default_difficulty_bits")]
    difficulty_bits: u32,
}

fn default_difficulty_bits() -> u32 {
    20
}

fn usage() -> ! {
    eprintln!(
        "Usage: ql-miner-multicore [--cuda] [--cuda-device ID] [-j WORKERS] NODE_ADDRESS:PORT YOUR_WALLET_ADDRESS_HEX"
    );
    std::process::exit(2);
}

fn main() {
    let args = env::args().skip(1).collect::<Vec<String>>();
    let mut workers = num_cpus::get();
    let mut cuda_enabled = false;
    let mut cuda_device = 0i32;
    let mut positional = Vec::new();

    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "-j" | "--threads" => {
                i += 1;
                if i >= args.len() {
                    usage();
                }
                workers = args[i].parse::<usize>().unwrap_or_else(|_| usage()).max(1);
            }
            "--cuda" => cuda_enabled = true,
            "--cuda-device" => {
                i += 1;
                if i >= args.len() {
                    usage();
                }
                cuda_device = args[i].parse::<i32>().unwrap_or_else(|_| usage());
            }
            "-h" | "--help" => usage(),
            value => positional.push(value.to_string()),
        }
        i += 1;
    }

    if positional.len() != 2 {
        usage();
    }
    let node = positional.remove(0);
    let wallet_hex = positional.remove(0);
    if wallet_hex.len() != WALLET_HEX_LEN {
        usage();
    }

    #[cfg(not(feature = "cuda"))]
    if cuda_enabled {
        eprintln!("[MINER] CUDA requested, but this binary was built without --features cuda.");
        eprintln!("[MINER] Build it with: cargo build --release --features cuda");
        std::process::exit(2);
    }

    let wallet = hex::decode(&wallet_hex).unwrap_or_else(|_| {
        eprintln!("[MINER] That doesn't look like a valid address.");
        std::process::exit(2);
    });

    let stop = Arc::new(AtomicBool::new(false));
    {
        let stop = Arc::clone(&stop);
        ctrlc::set_handler(move || stop.store(true, Ordering::Relaxed))
            .expect("failed to install Ctrl+C handler");
    }

    let endpoints = node_endpoints(&node).unwrap_or_else(|err| {
        eprintln!("[MINER] Bad node address: {err}");
        std::process::exit(2);
    });
    println!("[CONFIG] Connecting to node: {node}");
    println!("[MINER] Mining rewards will be paid to: {wallet_hex}");
    if cuda_enabled {
        println!("[MINER] Using CUDA device {cuda_device}. Press Ctrl+C to stop.");
    } else {
        println!("[MINER] Using {workers} worker thread(s). Press Ctrl+C to stop.");
    }

    let mut active_endpoint_label = String::new();
    let mut active_block = None;
    let miner_started = Instant::now();
    let mut total_checked = 0u64;
    let mut last_dashboard = Instant::now();
    let mut total_blocks_found = 0u64;
    let mut blocks_found = Vec::new();

    while !stop.load(Ordering::Relaxed) {
        let (endpoint, template) = match get_template_from_any(&endpoints) {
            Ok(result) => result,
            Err(err) => {
                eprintln!("[MINER] Could not reach node at {node} - {err}");
                sleep_or_stop(&stop, Duration::from_secs(5));
                continue;
            }
        };

        let endpoint_label = endpoint.label();
        if endpoint_label != active_endpoint_label {
            println!("[CONFIG] RPC endpoint: {endpoint_label}");
            active_endpoint_label = endpoint_label;
        }

        let block_key = (template.block_height, template.difficulty_bits);
        if active_block != Some(block_key) {
            println!(
                "[MINER] Block {} | target: {} leading zero bit(s) | expected work: {} hashes",
                template.block_height,
                template.difficulty_bits,
                format_hashes(expected_hashes(template.difficulty_bits))
            );
            active_block = Some(block_key);
        }

        if cuda_enabled {
            match mine_cuda_batch(&template, &wallet, cuda_device) {
                Ok(result) => {
                    total_checked = total_checked.saturating_add(result.checked);
                    let avg_rate =
                        total_checked as f64 / miner_started.elapsed().as_secs_f64().max(0.001);
                    let eta = expected_hashes(template.difficulty_bits) / avg_rate.max(0.001);

                    render_dashboard(&Dashboard {
                        mode: "CUDA",
                        device: format!("GPU {cuda_device}"),
                        endpoint: &active_endpoint_label,
                        block_height: template.block_height,
                        target_bits: template.difficulty_bits,
                        current_rate: result.rate,
                        average_rate: avg_rate,
                        checked: total_checked,
                        elapsed: miner_started.elapsed().as_secs_f64(),
                        eta,
                        start_nonce: result.start_nonce,
                        total_blocks_found,
                        blocks_found: &blocks_found,
                    });

                    if let Some(nonce) = result.nonce {
                        total_blocks_found = total_blocks_found.saturating_add(1);
                        let submit_status = match submit_nonce(&endpoint, &wallet_hex, nonce) {
                            Ok(body) => {
                                let lower = body.to_ascii_lowercase();
                                if lower.contains("accepted")
                                    || lower.contains("ok")
                                    || lower.contains("true")
                                {
                                    "accepted".to_string()
                                } else {
                                    format!("response: {}", compact_status(&body))
                                }
                            }
                            Err(err) => format!("submit failed: {err}"),
                        };

                        blocks_found.insert(
                            0,
                            FoundBlock {
                                block_height: template.block_height,
                                nonce,
                                status: submit_status,
                            },
                        );
                        blocks_found.truncate(8);

                        render_dashboard(&Dashboard {
                            mode: "CUDA",
                            device: format!("GPU {cuda_device}"),
                            endpoint: &active_endpoint_label,
                            block_height: template.block_height,
                            target_bits: template.difficulty_bits,
                            current_rate: result.rate,
                            average_rate: avg_rate,
                            checked: total_checked,
                            elapsed: miner_started.elapsed().as_secs_f64(),
                            eta,
                            start_nonce: result.start_nonce,
                            total_blocks_found,
                            blocks_found: &blocks_found,
                        });
                        println!(
                            "[MINER] SOLVED block {} | nonce={} | submit={}",
                            template.block_height,
                            nonce,
                            blocks_found
                                .first()
                                .map(|block| block.status.as_str())
                                .unwrap_or("unknown")
                        );
                    }
                }
                Err(err) => {
                    eprintln!("[CUDA] {err}");
                    sleep_or_stop(&stop, Duration::from_secs(5));
                }
            }
            continue;
        }

        let started = Instant::now();
        let mut last_status = Instant::now();
        let mut last_count = 0u64;
        let checked = Arc::new(AtomicU64::new(0));
        let found = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::channel();
        let mut handles = Vec::with_capacity(workers);
        let mut first_nonce = None;

        for worker_id in 0..workers {
            let template = template.clone();
            let wallet = wallet.clone();
            let tx = tx.clone();
            let stop = Arc::clone(&stop);
            let found = Arc::clone(&found);
            let checked = Arc::clone(&checked);

            handles.push(thread::spawn(move || {
                let mut rng = rand::thread_rng();
                let mut nonce = rng.next_u64().wrapping_add(worker_id as u64);
                if worker_id == 0 {
                    let _ = tx.send(MinerEvent::StartedAt(nonce));
                }

                for _ in 0..BATCH_PER_WORKER {
                    if stop.load(Ordering::Relaxed) || found.load(Ordering::Relaxed) {
                        return;
                    }

                    if valid_nonce(&template, &wallet, nonce) {
                        found.store(true, Ordering::Relaxed);
                        let _ = tx.send(MinerEvent::Found(nonce));
                        return;
                    }

                    nonce = nonce.wrapping_add(workers as u64);
                    checked.fetch_add(1, Ordering::Relaxed);
                }
            }));
        }

        drop(tx);

        let mut winning_nonce = None;
        loop {
            match rx.recv_timeout(Duration::from_millis(250)) {
                Ok(MinerEvent::StartedAt(nonce)) => {
                    first_nonce = Some(nonce);
                }
                Ok(MinerEvent::Found(nonce)) => {
                    winning_nonce = Some(nonce);
                    break;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    let count = checked.load(Ordering::Relaxed);
                    if last_status.elapsed() >= STATUS_INTERVAL {
                        let interval = last_status.elapsed().as_secs_f64();
                        let instant_rate = (count.saturating_sub(last_count)) as f64 / interval;
                        let global_checked = total_checked.saturating_add(count);
                        let average_rate = global_checked as f64
                            / miner_started.elapsed().as_secs_f64().max(0.001);
                        let expected = expected_hashes(template.difficulty_bits);
                        let eta = if average_rate > 0.0 {
                            expected / average_rate
                        } else {
                            f64::INFINITY
                        };

                        if last_dashboard.elapsed() >= STATUS_INTERVAL {
                            render_dashboard(&Dashboard {
                                mode: "CPU",
                                device: format!("{workers} threads"),
                                endpoint: &active_endpoint_label,
                                block_height: template.block_height,
                                target_bits: template.difficulty_bits,
                                current_rate: instant_rate,
                                average_rate,
                                checked: global_checked,
                                elapsed: miner_started.elapsed().as_secs_f64(),
                                eta,
                                start_nonce: first_nonce.unwrap_or(0),
                                total_blocks_found,
                                blocks_found: &blocks_found,
                            });
                            last_dashboard = Instant::now();
                        }
                        last_status = Instant::now();
                        last_count = count;
                    }
                    if stop.load(Ordering::Relaxed) {
                        break;
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }

        for handle in handles {
            let _ = handle.join();
        }
        total_checked = total_checked.saturating_add(checked.load(Ordering::Relaxed));

        if let Some(nonce) = winning_nonce {
            total_blocks_found = total_blocks_found.saturating_add(1);
            let submit_status = match submit_nonce(&endpoint, &wallet_hex, nonce) {
                Ok(body) => {
                    let lower = body.to_ascii_lowercase();
                    if lower.contains("accepted") || lower.contains("ok") || lower.contains("true")
                    {
                        "accepted".to_string()
                    } else {
                        format!("response: {}", compact_status(&body))
                    }
                }
                Err(err) => format!("submit failed: {err}"),
            };
            blocks_found.insert(
                0,
                FoundBlock {
                    block_height: template.block_height,
                    nonce,
                    status: submit_status,
                },
            );
            blocks_found.truncate(8);

            let avg_rate = total_checked as f64 / miner_started.elapsed().as_secs_f64().max(0.001);
            render_dashboard(&Dashboard {
                mode: "CPU",
                device: format!("{workers} threads"),
                endpoint: &active_endpoint_label,
                block_height: template.block_height,
                target_bits: template.difficulty_bits,
                current_rate: checked.load(Ordering::Relaxed) as f64
                    / started.elapsed().as_secs_f64().max(0.001),
                average_rate: avg_rate,
                checked: total_checked,
                elapsed: miner_started.elapsed().as_secs_f64(),
                eta: expected_hashes(template.difficulty_bits) / avg_rate.max(0.001),
                start_nonce: first_nonce.unwrap_or(0),
                total_blocks_found,
                blocks_found: &blocks_found,
            });
        }
    }
}

enum MinerEvent {
    StartedAt(u64),
    Found(u64),
}

struct FoundBlock {
    block_height: u64,
    nonce: u64,
    status: String,
}

struct Dashboard<'a> {
    mode: &'a str,
    device: String,
    endpoint: &'a str,
    block_height: u64,
    target_bits: u32,
    current_rate: f64,
    average_rate: f64,
    checked: u64,
    elapsed: f64,
    eta: f64,
    start_nonce: u64,
    total_blocks_found: u64,
    blocks_found: &'a [FoundBlock],
}

fn render_dashboard(dashboard: &Dashboard) {
    print!("\x1b[2J\x1b[H");
    println!("QL MINER - SRB STYLE STATUS");
    println!("==============================================================");
    println!("{:<18} {:<18} {:<18}", "Mode", "Device", "RPC Endpoint");
    println!(
        "{:<18} {:<18} {:<18}",
        dashboard.mode, dashboard.device, dashboard.endpoint
    );
    println!("--------------------------------------------------------------");
    println!(
        "{:<14} {:<18} {:<18} {:<18}",
        "Block", "Target", "Current H/s", "Average H/s"
    );
    println!(
        "{:<14} {:<18} {:<18} {:<18}",
        dashboard.block_height,
        format!("{} bits", dashboard.target_bits),
        format_rate(dashboard.current_rate),
        format_rate(dashboard.average_rate)
    );
    println!("--------------------------------------------------------------");
    println!(
        "{:<18} {:<18} {:<18} {:<18}",
        "Checked", "ETA", "Uptime", "Start Nonce"
    );
    println!(
        "{:<18} {:<18} {:<18} {:<18}",
        format_hashes(dashboard.checked as f64),
        format_duration(dashboard.eta),
        format_duration(dashboard.elapsed),
        dashboard.start_nonce
    );
    println!("--------------------------------------------------------------");
    println!("Blocks Found: {}", dashboard.total_blocks_found);
    println!("{:<10} {:<22} {:<28}", "Block", "Nonce", "Status");
    if dashboard.blocks_found.is_empty() {
        println!("{:<10} {:<22} {:<28}", "-", "-", "none yet");
    } else {
        for block in dashboard.blocks_found.iter().take(8) {
            println!(
                "{:<10} {:<22} {:<28}",
                block.block_height,
                block.nonce,
                truncate_for_table(&block.status, 28)
            );
        }
    }
    println!("==============================================================");
    println!("Ctrl+C to stop. Dashboard refreshes in-place.");
    let _ = io::stdout().flush();
}

fn compact_status(status: &str) -> String {
    status.split_whitespace().collect::<Vec<&str>>().join(" ")
}

fn truncate_for_table(value: &str, max_len: usize) -> String {
    if value.len() <= max_len {
        value.to_string()
    } else if max_len <= 3 {
        value[..max_len].to_string()
    } else {
        format!("{}...", &value[..max_len - 3])
    }
}

struct CudaBatchResult {
    nonce: Option<u64>,
    checked: u64,
    elapsed: f64,
    rate: f64,
    start_nonce: u64,
}

#[cfg(feature = "cuda")]
fn mine_cuda_batch(
    template: &Template,
    wallet: &[u8],
    device_id: i32,
) -> Result<CudaBatchResult, String> {
    let mut rng = rand::thread_rng();
    let start_nonce = rng.next_u64();
    let started = Instant::now();
    let mut found_nonce = 0u64;
    let mut checked = 0u64;

    let status = unsafe {
        ql_cuda_mine(
            template.previous_block_hash.as_ptr(),
            template.previous_block_hash.len(),
            template.merkle_root.as_ptr(),
            template.merkle_root.len(),
            template.block_height,
            wallet.as_ptr(),
            wallet.len(),
            template.difficulty_bits,
            start_nonce,
            CUDA_BATCH_NONCES,
            &mut found_nonce,
            &mut checked,
            device_id,
        )
    };

    let elapsed = started.elapsed().as_secs_f64();
    let rate = checked as f64 / elapsed.max(0.001);

    match status {
        1 => Ok(CudaBatchResult {
            nonce: Some(found_nonce),
            checked,
            elapsed,
            rate,
            start_nonce,
        }),
        0 => Ok(CudaBatchResult {
            nonce: None,
            checked,
            elapsed,
            rate,
            start_nonce,
        }),
        code => Err(format!("CUDA miner failed with code {code}")),
    }
}

#[cfg(not(feature = "cuda"))]
fn mine_cuda_batch(
    _template: &Template,
    _wallet: &[u8],
    _device_id: i32,
) -> Result<CudaBatchResult, String> {
    Err("CUDA support is not compiled into this binary".to_string())
}

#[derive(Clone)]
struct Endpoint {
    scheme: Scheme,
    host: String,
    port: u16,
}

#[derive(Clone, Copy)]
enum Scheme {
    Tls,
    Plain,
}

impl Endpoint {
    fn label(&self) -> String {
        let scheme = match self.scheme {
            Scheme::Tls => "tls",
            Scheme::Plain => "tcp",
        };
        format!("{scheme}://{}:{}", self.host, self.port)
    }
}

fn node_endpoints(node: &str) -> Result<Vec<Endpoint>, String> {
    let node = node.trim().trim_end_matches('/');
    if let Some(rest) = node.strip_prefix("https://") {
        let (host, port) = parse_host_port(rest)?;
        return Ok(vec![Endpoint {
            scheme: Scheme::Tls,
            host,
            port,
        }]);
    }
    if let Some(rest) = node.strip_prefix("http://") {
        let (host, port) = parse_host_port(rest)?;
        return Ok(vec![Endpoint {
            scheme: Scheme::Plain,
            host,
            port,
        }]);
    }

    let (host, port) = parse_host_port(node)?;
    let plain = Endpoint {
        scheme: Scheme::Plain,
        host: host.clone(),
        port,
    };
    let tls = Endpoint {
        scheme: Scheme::Tls,
        host: host.clone(),
        port: 443,
    };

    if host == "localhost" || host == "127.0.0.1" {
        Ok(vec![plain, tls])
    } else {
        Ok(vec![tls, plain])
    }
}

fn parse_host_port(input: &str) -> Result<(String, u16), String> {
    let input = input.trim_end_matches('/');
    let (host, port) = input
        .rsplit_once(':')
        .ok_or_else(|| format!("expected NODE_ADDRESS:PORT, got {input}"))?;
    if host.is_empty() {
        return Err("host is empty".to_string());
    }
    let port = port
        .parse::<u16>()
        .map_err(|_| format!("invalid port in {input}"))?;
    Ok((host.to_string(), port))
}

fn get_template_from_any(endpoints: &[Endpoint]) -> Result<(Endpoint, Template), String> {
    let mut errors = Vec::new();

    for endpoint in endpoints {
        match get_template(endpoint) {
            Ok(template) => return Ok((endpoint.clone(), template)),
            Err(err) => errors.push(format!("{}: {err}", endpoint.label())),
        }
    }

    Err(errors.join(" | ").into())
}

fn get_template(endpoint: &Endpoint) -> Result<Template, String> {
    let body = rpc_request(endpoint, "GET", "/api/mining/template", None)?;
    let response: TemplateResponse = serde_json::from_str(&body)
        .map_err(|err| format!("bad template JSON: {err}; body={body}"))?;

    Ok(Template {
        block_height: response.block_height,
        previous_block_hash: hex::decode(response.previous_block_hash)
            .map_err(|err| format!("bad previous_block_hash: {err}"))?,
        merkle_root: hex::decode(response.merkle_root)
            .map_err(|err| format!("bad merkle_root: {err}"))?,
        difficulty_bits: response.difficulty_bits,
    })
}

fn submit_nonce(endpoint: &Endpoint, wallet: &str, nonce: u64) -> Result<String, String> {
    let body = serde_json::json!({
        "miner": wallet,
        "nonce": nonce,
    })
    .to_string();

    rpc_request(endpoint, "POST", "/api/mining/submit", Some(&body))
}

fn rpc_request(
    endpoint: &Endpoint,
    method: &str,
    path: &str,
    body: Option<&str>,
) -> Result<String, String> {
    let request = build_http_request(endpoint, method, path, body);
    let response = match endpoint.scheme {
        Scheme::Plain => send_plain(endpoint, &request),
        Scheme::Tls => send_tls(endpoint, &request),
    }?;

    parse_http_response(&response)
}

fn build_http_request(
    endpoint: &Endpoint,
    method: &str,
    path: &str,
    body: Option<&str>,
) -> Vec<u8> {
    let body = body.unwrap_or("");
    let mut request = format!(
        "{method} {path} HTTP/1.1\r\nHost: {}\r\nUser-Agent: ql-miner-multicore/0.1\r\nAccept: application/json\r\nConnection: close\r\n",
        endpoint.host
    );

    if !body.is_empty() {
        request.push_str("Content-Type: application/json\r\n");
        request.push_str(&format!("Content-Length: {}\r\n", body.len()));
    }

    request.push_str("\r\n");
    request.push_str(body);
    request.into_bytes()
}

fn send_plain(endpoint: &Endpoint, request: &[u8]) -> Result<Vec<u8>, String> {
    let mut stream = connect_tcp(endpoint)?;
    stream
        .set_read_timeout(Some(Duration::from_secs(20)))
        .map_err(|err| format!("set read timeout: {err}"))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(20)))
        .map_err(|err| format!("set write timeout: {err}"))?;
    stream
        .write_all(request)
        .map_err(|err| format!("tcp write: {err}"))?;

    read_response(stream, "tcp")
}

fn send_tls(endpoint: &Endpoint, request: &[u8]) -> Result<Vec<u8>, String> {
    let tcp = connect_tcp(endpoint)?;
    tcp.set_read_timeout(Some(Duration::from_secs(20)))
        .map_err(|err| format!("set read timeout: {err}"))?;
    tcp.set_write_timeout(Some(Duration::from_secs(20)))
        .map_err(|err| format!("set write timeout: {err}"))?;

    let mut roots = RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    let config = ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    let server_name = ServerName::try_from(endpoint.host.clone())
        .map_err(|err| format!("bad TLS server name {}: {err}", endpoint.host))?;
    let connection = ClientConnection::new(Arc::new(config), server_name)
        .map_err(|err| format!("tls: {err}"))?;
    let mut stream = StreamOwned::new(connection, tcp);

    stream
        .write_all(request)
        .map_err(|err| format!("tls write: {err}"))?;

    read_response(stream, "tls")
}

fn connect_tcp(endpoint: &Endpoint) -> Result<TcpStream, String> {
    let address = format!("{}:{}", endpoint.host, endpoint.port);
    let addresses = address
        .to_socket_addrs()
        .map_err(|err| format!("resolve {address}: {err}"))?
        .collect::<Vec<SocketAddr>>();

    if addresses.is_empty() {
        return Err(format!("resolve {address}: no addresses"));
    }

    let mut errors = Vec::new();
    for socket_addr in addresses {
        match TcpStream::connect_timeout(&socket_addr, CONNECT_TIMEOUT) {
            Ok(stream) => return Ok(stream),
            Err(err) => errors.push(format!("{socket_addr}: {err}")),
        }
    }

    Err(format!(
        "tcp connect {address} timed out/failed after {}s: {}",
        CONNECT_TIMEOUT.as_secs(),
        errors.join(" | ")
    ))
}

fn read_response<R: Read>(mut stream: R, transport: &str) -> Result<Vec<u8>, String> {
    let mut response = Vec::new();
    let mut buffer = [0u8; 16 * 1024];

    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => response.extend_from_slice(&buffer[..n]),
            Err(err) if err.kind() == io::ErrorKind::UnexpectedEof && !response.is_empty() => {
                break;
            }
            Err(err) => return Err(format!("{transport} read: {err}")),
        }
    }

    Ok(response)
}

fn parse_http_response(response: &[u8]) -> Result<String, String> {
    let split = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| format!("invalid HTTP response: {} bytes", response.len()))?;
    let headers = std::str::from_utf8(&response[..split])
        .map_err(|err| format!("response headers are not UTF-8: {err}"))?;
    let body = &response[split + 4..];
    let mut lines = headers.lines();
    let status = lines.next().unwrap_or("");
    let status_code = status
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse::<u16>().ok())
        .ok_or_else(|| format!("invalid HTTP status line: {status}"))?;

    let is_chunked = lines.clone().any(|line| {
        let line = line.to_ascii_lowercase();
        line.starts_with("transfer-encoding:") && line.contains("chunked")
    });
    let body = if is_chunked {
        decode_chunked_body(body)?
    } else {
        body.to_vec()
    };
    let body =
        String::from_utf8(body).map_err(|err| format!("response body is not UTF-8: {err}"))?;

    if !(200..300).contains(&status_code) {
        return Err(format!("HTTP {status_code}: {body}"));
    }

    Ok(body)
}

fn decode_chunked_body(mut input: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();

    loop {
        let line_end = input
            .windows(2)
            .position(|window| window == b"\r\n")
            .ok_or_else(|| "bad chunked response: missing chunk size".to_string())?;
        let size_line = std::str::from_utf8(&input[..line_end])
            .map_err(|err| format!("bad chunk size UTF-8: {err}"))?;
        let size_hex = size_line.split(';').next().unwrap_or(size_line).trim();
        let size = usize::from_str_radix(size_hex, 16)
            .map_err(|err| format!("bad chunk size {size_hex}: {err}"))?;
        input = &input[line_end + 2..];

        if size == 0 {
            return Ok(out);
        }
        if input.len() < size + 2 {
            return Err("bad chunked response: chunk shorter than declared".to_string());
        }

        out.extend_from_slice(&input[..size]);
        input = &input[size + 2..];
    }
}

fn valid_nonce(template: &Template, wallet: &[u8], nonce: u64) -> bool {
    let mut hasher = Sha3_256::new();
    hasher.update(1u32.to_le_bytes());
    hasher.update(&template.previous_block_hash);
    hasher.update(&template.merkle_root);
    hasher.update(template.block_height.to_le_bytes());
    hasher.update(wallet);
    hasher.update(nonce.to_le_bytes());

    has_leading_zero_bits(&hasher.finalize(), template.difficulty_bits)
}

fn has_leading_zero_bits(bytes: &[u8], bits: u32) -> bool {
    let full_bytes = (bits / 8) as usize;
    let partial_bits = (bits % 8) as u8;

    if full_bytes > bytes.len() {
        return false;
    }

    if bytes[..full_bytes].iter().any(|&byte| byte != 0) {
        return false;
    }

    if partial_bits == 0 {
        return true;
    }

    if full_bytes == bytes.len() {
        return false;
    }

    bytes[full_bytes] >> (8 - partial_bits) == 0
}

fn sleep_or_stop(stop: &AtomicBool, duration: Duration) {
    let deadline = Instant::now() + duration;
    while Instant::now() < deadline && !stop.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_millis(100));
    }
}

fn expected_hashes(difficulty_bits: u32) -> f64 {
    2f64.powi(difficulty_bits.min(255) as i32)
}

fn format_target(difficulty_bits: u32) -> String {
    let full_zero_bytes = difficulty_bits / 8;
    let extra_bits = difficulty_bits % 8;

    if extra_bits == 0 {
        format!("{difficulty_bits} bits ({full_zero_bytes} zero byte(s))")
    } else {
        format!("{difficulty_bits} bits ({full_zero_bytes} zero byte(s) + {extra_bits} bit(s))")
    }
}

fn format_hashes(value: f64) -> String {
    format_scaled(value, "H")
}

fn format_rate(value: f64) -> String {
    format!("{}/s", format_scaled(value, "H"))
}

fn format_scaled(value: f64, suffix: &str) -> String {
    const UNITS: [&str; 6] = ["", "k", "M", "G", "T", "P"];
    let mut scaled = value.max(0.0);
    let mut unit = 0usize;

    while scaled >= 1000.0 && unit < UNITS.len() - 1 {
        scaled /= 1000.0;
        unit += 1;
    }

    if unit == 0 {
        format!("{scaled:.0} {suffix}")
    } else {
        format!("{scaled:.2} {}{suffix}", UNITS[unit])
    }
}

fn format_duration(seconds: f64) -> String {
    if !seconds.is_finite() {
        return "unknown".to_string();
    }

    let seconds = seconds.max(0.0);
    if seconds < 60.0 {
        format!("{seconds:.1}s")
    } else if seconds < 3600.0 {
        format!("{:.1}m", seconds / 60.0)
    } else if seconds < 86_400.0 {
        format!("{:.1}h", seconds / 3600.0)
    } else {
        format!("{:.1}d", seconds / 86_400.0)
    }
}
