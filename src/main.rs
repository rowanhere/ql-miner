use rand::RngCore;
use reqwest::blocking::Client;
use serde::Deserialize;
use sha3::{Digest, Sha3_256};
use std::env;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant};

const WALLET_HEX_LEN: usize = 3904;
const BATCH_PER_WORKER: u64 = 250_000;
const STATUS_INTERVAL: Duration = Duration::from_secs(2);

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
    eprintln!("Usage: ql-miner-multicore [-j WORKERS] NODE_ADDRESS:PORT YOUR_WALLET_ADDRESS_HEX");
    std::process::exit(2);
}

fn main() {
    let mut args = env::args().skip(1);
    let mut workers = num_cpus::get();
    let first = args.next().unwrap_or_else(|| usage());

    let node = if first == "-j" || first == "--threads" {
        let count = args.next().unwrap_or_else(|| usage());
        workers = count.parse::<usize>().unwrap_or_else(|_| usage()).max(1);
        args.next().unwrap_or_else(|| usage())
    } else {
        first
    };

    let wallet_hex = args.next().unwrap_or_else(|| usage());
    if args.next().is_some() || wallet_hex.len() != WALLET_HEX_LEN {
        usage();
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

    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent("ql-miner-multicore/0.1")
        .build()
        .expect("failed to build HTTP client");

    let base_urls = node_base_urls(&node);
    println!("[CONFIG] Connecting to node: {node}");
    println!("[MINER] Mining rewards will be paid to: {wallet_hex}");
    println!("[MINER] Using {workers} worker thread(s). Press Ctrl+C to stop.");

    while !stop.load(Ordering::Relaxed) {
        let (base_url, template) = match get_template_from_any(&client, &base_urls) {
            Ok(result) => result,
            Err(err) => {
                eprintln!("[MINER] Could not reach node at {node} - {err}");
                sleep_or_stop(&stop, Duration::from_secs(5));
                continue;
            }
        };

        println!(
            "[MINER] Block {} | target: {} leading zero bit(s) | expected work: {} hashes",
            template.block_height,
            template.difficulty_bits,
            format_hashes(expected_hashes(template.difficulty_bits))
        );

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
                    println!(
                        "[MINER] Searching nonce stream from {} across {} worker(s)",
                        nonce, workers
                    );
                }
                Ok(MinerEvent::Found(nonce)) => {
                    winning_nonce = Some(nonce);
                    break;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    let count = checked.load(Ordering::Relaxed);
                    if last_status.elapsed() >= STATUS_INTERVAL {
                        let elapsed = started.elapsed().as_secs_f64();
                        let interval = last_status.elapsed().as_secs_f64();
                        let instant_rate = (count.saturating_sub(last_count)) as f64 / interval;
                        let average_rate = count as f64 / elapsed.max(0.001);
                        let expected = expected_hashes(template.difficulty_bits);
                        let eta = if average_rate > 0.0 {
                            expected / average_rate
                        } else {
                            f64::INFINITY
                        };

                        print!(
                            "\r[MINER] block={} target={} checked={} rate={} avg={} elapsed={} eta~{} start_nonce={}",
                            template.block_height,
                            format_target(template.difficulty_bits),
                            format_hashes(count as f64),
                            format_rate(instant_rate),
                            format_rate(average_rate),
                            format_duration(elapsed),
                            format_duration(eta),
                            first_nonce
                                .map(|nonce| nonce.to_string())
                                .unwrap_or_else(|| "pending".to_string())
                        );
                        let _ = io::stdout().flush();
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
        println!();

        if let Some(nonce) = winning_nonce {
            println!(
                "[MINER] SOLVED block {} | nonce={} | checked={} | elapsed={} | avg={}",
                template.block_height,
                nonce,
                format_hashes(checked.load(Ordering::Relaxed) as f64),
                format_duration(started.elapsed().as_secs_f64()),
                format_rate(
                    checked.load(Ordering::Relaxed) as f64
                        / started.elapsed().as_secs_f64().max(0.001),
                )
            );
            match submit_nonce(&client, &base_url, &wallet_hex, nonce) {
                Ok(body) => {
                    let lower = body.to_ascii_lowercase();
                    if lower.contains("accepted") || lower.contains("ok") || lower.contains("true")
                    {
                        println!("[MINER] SUBMIT accepted: {body}");
                    } else {
                        println!("[MINER] SUBMIT response: {body}");
                    }
                }
                Err(err) => eprintln!("[MINER] Failed to submit - node unreachable - {err}"),
            }
        } else if !stop.load(Ordering::Relaxed) {
            println!(
                "[MINER] Batch finished | checked={} | avg={} | no match, refetching",
                format_hashes(checked.load(Ordering::Relaxed) as f64),
                format_rate(
                    checked.load(Ordering::Relaxed) as f64
                        / started.elapsed().as_secs_f64().max(0.001),
                )
            );
        }
    }
}

enum MinerEvent {
    StartedAt(u64),
    Found(u64),
}

fn node_base_urls(node: &str) -> Vec<String> {
    if node.starts_with("http://") || node.starts_with("https://") {
        vec![node.trim_end_matches('/').to_string()]
    } else {
        let node = node.trim_end_matches('/');
        vec![format!("https://{node}"), format!("http://{node}")]
    }
}

fn get_template_from_any(
    client: &Client,
    base_urls: &[String],
) -> Result<(String, Template), Box<dyn std::error::Error>> {
    let mut errors = Vec::new();

    for base_url in base_urls {
        match get_template(client, base_url) {
            Ok(template) => {
                println!("[CONFIG] RPC endpoint: {base_url}");
                return Ok((base_url.clone(), template));
            }
            Err(err) => errors.push(format!("{base_url}: {err}")),
        }
    }

    Err(errors.join(" | ").into())
}

fn get_template(client: &Client, base_url: &str) -> Result<Template, Box<dyn std::error::Error>> {
    let response: TemplateResponse = client
        .get(format!("{base_url}/api/mining/template"))
        .send()?
        .error_for_status()?
        .json()?;

    Ok(Template {
        block_height: response.block_height,
        previous_block_hash: hex::decode(response.previous_block_hash)?,
        merkle_root: hex::decode(response.merkle_root)?,
        difficulty_bits: response.difficulty_bits,
    })
}

fn submit_nonce(
    client: &Client,
    base_url: &str,
    wallet: &str,
    nonce: u64,
) -> Result<String, Box<dyn std::error::Error>> {
    let response = client
        .post(format!("{base_url}/api/mining/submit"))
        .json(&serde_json::json!({
            "miner": wallet,
            "nonce": nonce,
        }))
        .send()?
        .error_for_status()?;

    Ok(response.text()?)
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
