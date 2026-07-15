# QL Multicore Miner

This is a source rewrite of the bundled QL miner loop. It uses one process with
multiple worker threads instead of spawning one miner process per CPU core.

## Build on Linux

```bash
cd ql-miner-multicore
cargo build --release
```

The binary will be created at:

```bash
target/release/ql-miner-multicore
```

## Run

```bash
./target/release/ql-miner-multicore NODE_ADDRESS:PORT YOUR_WALLET_ADDRESS_HEX
```

By default it uses all detected CPU threads. To pick a worker count:

```bash
./target/release/ql-miner-multicore -j 32 NODE_ADDRESS:PORT YOUR_WALLET_ADDRESS_HEX
```

The miner defaults to `https://NODE_ADDRESS:PORT`, matching the original
binary's TLS client behavior. You can pass an explicit `http://` or `https://`
URL if needed.
