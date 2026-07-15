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

    let base_url = normalize_node_url(&node);
    println!("[CONFIG] Connecting to node: {node}");
    println!("[MINER] Mining rewards will be paid to: {wallet_hex}");
    println!("[MINER] Using {workers} worker thread(s). Press Ctrl+C to stop.");

    while !stop.load(Ordering::Relaxed) {
        let template = match get_template(&client, &base_url) {
            Ok(template) => template,
            Err(err) => {
                eprintln!("[MINER] Could not reach node at {node} - {err}");
                sleep_or_stop(&stop, Duration::from_secs(5));
                continue;
            }
        };

        println!(
            "[MINER] Got template for block {} mining at {} bits",
            template.block_height, template.difficulty_bits
        );

        let started = Instant::now();
        let checked = Arc::new(AtomicU64::new(0));
        let found = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::channel();
        let mut handles = Vec::with_capacity(workers);

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

                for _ in 0..BATCH_PER_WORKER {
                    if stop.load(Ordering::Relaxed) || found.load(Ordering::Relaxed) {
                        return;
                    }

                    if valid_nonce(&template, &wallet, nonce) {
                        found.store(true, Ordering::Relaxed);
                        let _ = tx.send(nonce);
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
            match rx.recv_timeout(Duration::from_secs(1)) {
                Ok(nonce) => {
                    winning_nonce = Some(nonce);
                    break;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    let count = checked.load(Ordering::Relaxed);
                    print!(
                        "\r[MINER] Checked {count} nonces in {:.1}s",
                        started.elapsed().as_secs_f32()
                    );
                    let _ = io::stdout().flush();
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
            println!("[MINER] FOUND nonce {nonce}");
            match submit_nonce(&client, &base_url, &wallet_hex, nonce) {
                Ok(body) => println!("[MINER] Submit response: {body}"),
                Err(err) => eprintln!("[MINER] Failed to submit - node unreachable - {err}"),
            }
        } else if !stop.load(Ordering::Relaxed) {
            println!(
                "[MINER] Batch of {} nonces finished, no match, refetching",
                checked.load(Ordering::Relaxed)
            );
        }
    }
}

fn normalize_node_url(node: &str) -> String {
    if node.starts_with("http://") || node.starts_with("https://") {
        node.trim_end_matches('/').to_string()
    } else {
        format!("https://{}", node.trim_end_matches('/'))
    }
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
