# QL CUDA/CPU Miner

Source miner for Quantum Lattice with a terminal dashboard, CPU mining, and a
pure CUDA GPU miner.

## Fresh VPS Quick Start

Use a CUDA **devel** image when renting a GPU VPS. Runtime-only images usually
have `nvidia-smi` but do not have `nvcc`, so they cannot compile the CUDA miner.

```bash
apt update
apt install -y git
git clone https://github.com/rowanhere/ql-miner.git
cd ql-miner
chmod +x run-vps.sh
./run-vps.sh
```

The script will:

- install common build libraries on apt-based Linux systems
- install Rust if `cargo` is missing
- auto-detect NVIDIA GPUs with `nvidia-smi`
- use all detected GPUs by default
- infer common CUDA architectures such as RTX 4090 and RTX 5090
- build and run the miner

## Use Your Own Wallet

The script includes the current default wallet, but other users should override
it:

```bash
QL_WALLET='<your_wallet_hex>' ./run-vps.sh
```

To keep it for the current shell:

```bash
export QL_WALLET='<your_wallet_hex>'
./run-vps.sh
```

## Common Commands

Auto-detect and use all GPUs:

```bash
./run-vps.sh
```

Force all 4 RTX 5090 GPUs:

```bash
CUDA_ARCH=sm_120 CUDA_THREADS=128 CUDA_DEVICES=0,1,2,3 ./run-vps.sh
```

Force all 8 GPUs:

```bash
CUDA_DEVICES=0,1,2,3,4,5,6,7 ./run-vps.sh
```

Run CPU mining only:

```bash
./run-vps.sh --cpu
```

Mine against a different node:

```bash
./run-vps.sh quantum-lattice.futuristicai.co.za:8034
```

## CUDA Tuning

Defaults are tuned for the tested RTX 5090 VPS setup:

```bash
CUDA_THREADS=128
```

Optional tuning knobs:

```bash
CUDA_THREADS=256 ./run-vps.sh
CUDA_MIN_BLOCKS=2 ./run-vps.sh
CUDA_MAXRREGCOUNT=64 ./run-vps.sh
```

If a tuning value lowers `Current H/s`, stop it and go back to the default.

Common architecture overrides:

```bash
CUDA_ARCH=sm_120 ./run-vps.sh   # RTX 5090
CUDA_ARCH=sm_89 ./run-vps.sh    # RTX 4090, L40, L4
CUDA_ARCH=sm_86 ./run-vps.sh    # RTX 30xx, RTX A-series Ampere
CUDA_ARCH=sm_80 ./run-vps.sh    # A100
CUDA_ARCH=sm_90 ./run-vps.sh    # H100/H200
```

## Manual Build

CPU:

```bash
cargo build --release
./target/release/ql-miner-multicore -j "$(nproc)" quantum-lattice.futuristicai.co.za:8034 "$QL_WALLET"
```

CUDA:

```bash
cargo build --release --features cuda
./target/release/ql-miner-multicore --cuda --cuda-devices 0,1 quantum-lattice.futuristicai.co.za:8034 "$QL_WALLET"
```

## Dashboard

The miner prints a clear VPS-friendly dashboard:

- block height and target difficulty
- current and average hashrate
- checked hashes and ETA
- shortened miner address
- latest 8 found/accepted block rows

Balance only increases when `Accepted Blocks` increases and the row status says
`accepted`.

## Network Behavior

Use the same bare `NODE_ADDRESS:PORT` shape as the original binary. The miner
sends HTTP/1.1 requests to `/api/mining/template` and `/api/mining/submit`. For
remote bare hosts it tries TLS on port `443` first, then plain HTTP on the typed
port as fallback.
