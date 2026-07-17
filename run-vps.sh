#!/usr/bin/env bash
set -euo pipefail

NODE="${NODE:-quantum-lattice.futuristicai.co.za:8034}"
THREADS="${THREADS:-}"
CUDA="${CUDA:-auto}"
CUDA_DEVICE="${CUDA_DEVICE:-}"
CUDA_DEVICES="${CUDA_DEVICES:-}"
CUDA_ARCH="${CUDA_ARCH:-auto}"
CUDA_THREADS="${CUDA_THREADS:-128}"
CUDA_MIN_BLOCKS="${CUDA_MIN_BLOCKS:-}"
CUDA_MAXRREGCOUNT="${CUDA_MAXRREGCOUNT:-}"
QL_WALLET="${QL_WALLET:-9137dc2d1b41301e6b43da5ee4527e4ea0c9c1bb651b13919c3f526730660ffebbe891465123a6498fbe2624a459d59eb693acbeb91b09e5649361cc989bbda384cf93f802226175492461a54d890e0c1ae570b53d77c9aab05d7d9cc1cbd810addd17c83dc044dc5159a3ba0a371fb79887e6415164f5a1f316b3d8f38b4ab06377af90a6cfe5858a1ea494d5e4ebad8284015ad8c49578d367155c57c741ef29841dcea45ab195d9bf18363bd6a88fd0595eb2be3db7645df9e75dca02d1bf7dcacf3bbcd7011ec9b6f2af1f78db78d0b8e8ca7867c7414a355dac989b892b19e48b1fead3e8f58cc055028e1dd69fd539b2199748de4d671880cb79b0994f7a4cba4a42b0a12f07dd29f745becb4cda1876807bc727aa5fc09ff563ba2e8a3f868ef56d1a1236823e0bc9e59f0eb9257fb9c52652b7ce7059a6cfcc5da596a0d2e117f6b7cab9d809d65c8b0fb33b48e9d9b8943d1e0bbf3d14ca96d18ae5633ab5c9d126a170783380c7e489b1cb3c90e6e8b92e097f3351918733d49d91cb3791eef189f8835233bd186b0e0ad8814f5fa2190623f055691629073fbbb0c805ce0abcb543601dc13b48189a4816a47a17e3b1063324719a8c2a3c21cdf30bcd4c04047afb3d426ecaf58c6835071da50a9fff3a7207aace5f1ed4baf3a503225a583b17945a2419db50c88e109fc256933209e12c7a7d2c3af04b0dc1e1954d1feff428c992574df91ec84066b29aac365c0eb08222344ce0823962df6cac523b09037d0e0fe37fcbc45920ef4a3d97173b9566a7ba52cc9eeb16852d78bdb01e9f7729630cac9c1523eda57b9754089dc13ce3e2520ff409d79a139e5feb07214aaa1d151c21e1842f40a43825333ba6d13a659a5c6e603b4d8305f1dc25eca83771e00261eba642c340c6b260a1c7cc22d4baac194d33d4cd79ce7596520362ab797a6c5b555420f2b80730bbf093c9602c56420396746b6b2deb682ee3acef540bcf76ab3db7f988711c39099cbf4fea1da9f2caf1075362bbd625bb361aaad098f79eba4deb0ec1275bec412cb255f2a3f9c9b3ce78148fe4ddc7ef1e467233d0f2e91810120aaf2eaf08750fd6b4e2891f5ddf7787290f532b1c8b9e4f32911cfc4be6df939e639829ff59121645cdce92959ec2e87d5dc5cde4407bae505401cecd39889d7af5bf05ed9b2367ea3a11baec735002fc85788be25dcdb89b70881e1fb9cb7287a383e0bc660c8f9b84a71a3e8fed13113b1496dd422906e0ae89314e6383f346f714cd5d7c9bf154737a4ebb9d8a98fca305cdabf3693ad3d681706962026d937199c08e320d60484d7964edd1c15d1c64d0ba964ed91fc8612dccf858de3f9f5591ceee7ceb88dd6c72f72cd39563138e58d8d9be9b0e25c9e246d320ba2a3166d883acb8dbfbaa7a55ca6aa7e40662ec1536eff0fe49d8132bfec70244daa1d3cb1ec9c428033f7b8d5f847ed418fb07b5840cbec969fea53508d1cdcea07ecec6fdee886ed40491bf63200d518bdbf90533eb376b4ca5fa83dd45ad2f9161ce0675bbfe7e52cf02da432aa5f6059a0e5da8d2215f01a79f6429593fe8534f30636d9dc8093e3e34e6a0d3710b8627d95572872b32e88bf299c4bc23ff185eee097c390a392657c03a74dcddbd704836704424a548917a419d5257f0bad5e9bcaae6afeb6535b70b707a3d0f2b84a2fcb03f809116bd4e307f5f5fb1a1f7ac747e4116320bccef5389f129c805f0b40bdbb7d702e3f2d098f2551c8eac5012098cc0b9a9995476ee88fe349b1d971261c69a6e883e3e35980ac5c65006239cd2a56397b1c067e6761f3d4c5ac2601d18f9bbee27609822c77a8edf7ba4a5c81c0c2b34add8de24ff351730c253ea4543767f44e51c9e6035da433addca7f7ba935cef9b90b78cefd754a26ee3fa5df88d82c0ecc0ddbddd6282d2d723f8c8d0237f83bac20d5c1b5c8759d3fe9569b5c3e3e7061de71d8c10ff513a42bedcaa1794c990b0d9d50253200001a2b51e5c8fb4247867b9e4403928c3bf740e13878afbda00488cab7420b6b325eec01f5c5b8ee973885102b37a1f819478b27f5f1d6dea3eec9ebd5bb159b9fed75438cc9811df004b7b732a6b24cb4b4518b9eb2c09e6b19f482d042de15639fb60b9ae1bd16f54ba651cb505b373fd6e802cfb23c8a28c757ed120af169409ebadb8cfd1f74d8407b5949560a865a1d6f0bbbcbdb01ef840b55f8ec7f75bfcd3cc66153b273682cd437c0b918dd90c799667eb9e5487202d6aea9f4e37a914fdb1fc52e52765fc553b7a62c0b01dc6bd966cf879a46ddefd12a5cb2e7e60de375acb3ba47a2796fc7329d166d776f17b352723b457da0f386a5300ba3979929494f6e41ce6c8c793194a404c904b1e0b2389e761181990fcae77d022b655db4b041cb17a6b22ec05b3c5370ff5058ee599b27f705e1b6a82239e2d530353ce8159405277d36fa04989eb02a4b26c3b1e3d160d6261e6744fc31da67beb982659e59c7b7e7b1a195643d32ffcec9e9009a398ec3ff2448be7e9dc8cfc4aa457e0f45593e8268fbbd97f7a9cb9cba3c3250aebc00a32375dbd292e5d6603e7c9189bb87b9f194369665cee2625b7f0c6b4c8cddce7426abc5853a95d43ad91325a8d91da4766ab7f372239c6d9fa09227538c7e7c7dd463f713b2707f8dabf1ad909d659b041fe79b64bc874e952ff6ce657540cd9bd0c168178c27f13dddaa2b65354fd22e190b96}"

usage() {
  cat <<'USAGE'
Usage:
  ./run-vps.sh [--cuda|--cpu] [NODE_ADDRESS:PORT]

Fresh VPS quick start:
  git clone https://github.com/rowanhere/ql-miner.git
  cd ql-miner
  chmod +x run-vps.sh
  ./run-vps.sh

Defaults:
  NODE=quantum-lattice.futuristicai.co.za:8034
  CUDA=auto             # uses all NVIDIA GPUs when nvidia-smi + nvcc exist
  CUDA_THREADS=128      # fast default from 5090 testing
  QL_WALLET=embedded    # override with your own wallet env var

Useful overrides:
  QL_WALLET='<wallet_hex>' ./run-vps.sh
  CUDA_DEVICES=0,1,2,3 ./run-vps.sh
  CUDA_ARCH=sm_120 ./run-vps.sh       # RTX 5090
  CUDA_ARCH=sm_89 ./run-vps.sh        # RTX 4090/L40/L4
  ./run-vps.sh --cpu
USAGE
}

log() {
  echo "[RUN] $*"
}

have() {
  command -v "$1" >/dev/null 2>&1
}

as_root() {
  if [ "$(id -u)" -eq 0 ]; then
    "$@"
  elif have sudo; then
    sudo "$@"
  else
    echo "Need root or sudo to install packages: $*" >&2
    exit 1
  fi
}

install_apt_basics() {
  if ! have apt-get; then
    return
  fi

  local missing=()
  for cmd in git curl cc pkg-config; do
    if ! have "$cmd"; then
      missing+=("$cmd")
    fi
  done

  if [ "${#missing[@]}" -eq 0 ] && dpkg -s build-essential libssl-dev ca-certificates >/dev/null 2>&1; then
    return
  fi

  log "Installing VPS build dependencies..."
  as_root apt-get update
  as_root env DEBIAN_FRONTEND=noninteractive apt-get install -y \
    git curl ca-certificates build-essential pkg-config libssl-dev
}

install_rust_if_missing() {
  if have cargo; then
    return
  fi

  log "Installing Rust toolchain..."
  curl https://sh.rustup.rs -sSf | sh -s -- -y
  # shellcheck disable=SC1090
  . "$HOME/.cargo/env"
}

try_install_cuda_toolkit_if_missing() {
  if have nvcc; then
    return
  fi

  if have apt-get; then
    log "nvcc missing; trying apt CUDA toolkit install..."
    as_root apt-get update
    if as_root env DEBIAN_FRONTEND=noninteractive apt-get install -y nvidia-cuda-toolkit; then
      return
    fi
  fi

  cat >&2 <<'MSG'
[RUN] nvcc is still missing.
[RUN] Use a CUDA *devel* image on Vast/RunPod/etc, or install NVIDIA CUDA Toolkit.
[RUN] Runtime-only CUDA images have nvidia-smi but cannot compile this miner.
MSG
  exit 1
}

gpu_count() {
  if ! have nvidia-smi; then
    echo 0
    return
  fi
  nvidia-smi -L 2>/dev/null | grep -c '^GPU ' || true
}

gpu_names() {
  if have nvidia-smi; then
    nvidia-smi --query-gpu=name --format=csv,noheader 2>/dev/null || true
  fi
}

detect_cuda_devices() {
  local count
  count="$(gpu_count)"
  if [ "$count" -le 0 ]; then
    echo ""
    return
  fi

  local devices=""
  local i
  for ((i = 0; i < count; i++)); do
    if [ -z "$devices" ]; then
      devices="$i"
    else
      devices="$devices,$i"
    fi
  done
  echo "$devices"
}

detect_cuda_arch() {
  if [ "$CUDA_ARCH" != "auto" ]; then
    echo "$CUDA_ARCH"
    return
  fi

  local names
  names="$(gpu_names | tr '[:upper:]' '[:lower:]')"
  case "$names" in
    *5090*|*blackwell*) echo "sm_120" ;;
    *4090*|*4080*|*4070*|*4060*|*l40*|*l4*) echo "sm_89" ;;
    *3090*|*3080*|*3070*|*3060*|*a6000*|*a5000*|*a4000*) echo "sm_86" ;;
    *a100*) echo "sm_80" ;;
    *h100*|*h200*) echo "sm_90" ;;
    *) echo "sm_89" ;;
  esac
}

cpu_threads() {
  if [ -n "$THREADS" ]; then
    echo "$THREADS"
  elif have nproc; then
    nproc
  else
    echo 1
  fi
}

while [ "${1:-}" != "" ]; do
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    --cuda)
      CUDA=1
      ;;
    --cpu)
      CUDA=0
      ;;
    *)
      NODE="$1"
      ;;
  esac
  shift
done

install_apt_basics
install_rust_if_missing
# shellcheck disable=SC1090
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"

GPU_COUNT="$(gpu_count)"
if [ "$CUDA" = "auto" ]; then
  if [ "$GPU_COUNT" -gt 0 ]; then
    CUDA=1
  else
    CUDA=0
  fi
fi

if [ "$CUDA" = "1" ]; then
  if [ "$GPU_COUNT" -le 0 ]; then
    echo "[RUN] No NVIDIA GPUs detected by nvidia-smi; use --cpu or fix GPU drivers." >&2
    exit 1
  fi

  try_install_cuda_toolkit_if_missing

  if [ -z "$CUDA_DEVICES" ]; then
    if [ -n "$CUDA_DEVICE" ]; then
      CUDA_DEVICES="$CUDA_DEVICE"
    else
      CUDA_DEVICES="$(detect_cuda_devices)"
    fi
  fi

  export CUDA_ARCH
  CUDA_ARCH="$(detect_cuda_arch)"
  export CUDA_THREADS
  [ -n "$CUDA_MIN_BLOCKS" ] && export CUDA_MIN_BLOCKS
  [ -n "$CUDA_MAXRREGCOUNT" ] && export CUDA_MAXRREGCOUNT

  log "Detected GPUs: $GPU_COUNT"
  gpu_names | sed 's/^/[RUN] GPU: /'
  log "CUDA devices: $CUDA_DEVICES"
  log "CUDA arch: $CUDA_ARCH"
  log "CUDA threads: $CUDA_THREADS"
  log "Building release CUDA miner..."
  cargo build --release --features cuda

  log "Node: $NODE"
  log "Wallet: ${QL_WALLET:0:4}...${QL_WALLET: -4}"
  exec ./target/release/ql-miner-multicore --cuda --cuda-devices "$CUDA_DEVICES" "$NODE" "$QL_WALLET"
fi

THREADS="$(cpu_threads)"
log "Building release CPU miner..."
cargo build --release
log "Node: $NODE"
log "Threads: $THREADS"
log "Wallet: ${QL_WALLET:0:4}...${QL_WALLET: -4}"
exec ./target/release/ql-miner-multicore -j "$THREADS" "$NODE" "$QL_WALLET"
