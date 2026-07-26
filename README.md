# Xfeatures IPC

A lock-free, zero-copy Single-Producer Single-Consumer (SPSC) ring buffer for
Rust. Data moves between processes over a memory-mapped file (`mmap`) — once
the initial mapping is set up, no further syscalls are made on the hot path.

```
src/builder.rs      IpcBuilder — creates the shared-memory mapping, producer/consumer split
src/ring_buffer.rs  lock-free SPSC ring buffer, Acquire/Release index handoff
src/layout.rs       on-disk/shared-memory layout of the ring buffer header + slots
src/mmap.rs         mmap wrapper around the backing file
examples/basic_ipc.rs  single-process producer/consumer demo
benches/ipc_benchmark.rs  criterion benchmark vs. Unix Domain Sockets
server_benchmarks/  raw criterion HTML reports (ipc_push_pop, uds_push_pop)
deploy_bench.sh     provisions a bare Debian 12 box and runs `cargo bench`
```

## Design

- **Zero-copy** — data is written directly into the shared-memory slots, no
  intermediate buffers.
- **Lock-free** — no mutexes, no rwlocks, no spinlocks on the producer/consumer
  path.
- **Fail-fast** — a full buffer returns `TryPushError` immediately instead of
  busy-waiting.
- **Cache-line padding** (`#[repr(align(64))]`) — producer `head` and consumer
  `tail` sit on separate 64-byte lines, so false-sharing can't invalidate them
  across cores.
- **Acquire/Release, not SeqCst** — memory visibility without the pipeline
  stalls of a full sequential-consistency fence.
- **Local index caching** — each side caches the other's index and only
  touches the atomic across the memory bus once its local view says
  empty/full.
- **Unaligned-safe generics** — any `T: Copy` is serialized with
  `ptr::write_unaligned`/`read_unaligned`, so it's sound on strict-alignment
  targets (e.g. ARM), not just x86.

## Quick start

Add `xfeatures-ipc` to `Cargo.toml`, then:

```rust
use xfeatures_ipc::IpcBuilder;
use std::thread;

#[derive(Copy, Clone, Debug)]
#[repr(C)]
struct SensorData {
    timestamp_ms: u64,
    temperature: f32,
    humidity: f32,
}

fn main() {
    // Same-process demo. For separate processes, use build_producer()/build_consumer()
    // against the same name from each process instead of build().
    let (mut prod, mut cons) = IpcBuilder::<SensorData>::new("sensor_stream")
        .capacity(1024) // must be a power of two
        .build()
        .unwrap();

    let consumer_thread = thread::spawn(move || {
        while let Some(data) = cons.pop() {
            println!("Received: {:.1}C", data.temperature);
            break;
        }
    });

    let data = SensorData { timestamp_ms: 1_600_000_000, temperature: 22.5, humidity: 45.0 };
    prod.push(data).expect("Buffer full"); // fail-fast, no blocking

    consumer_thread.join().unwrap();
}
```

Run the fuller producer/consumer demo:

```bash
cargo run --example basic_ipc
```

## Benchmarks

Measured on Debian 12 (server-grade hardware), Xfeatures IPC vs. non-blocking
Unix Domain Sockets:

| | Latency | Throughput |
|---|---|---|
| Xfeatures IPC | 8.9 ns/msg | ~112,000,000 msg/sec |
| Unix Domain Sockets | ~511 ns/msg | — |

That's roughly 57x lower latency than UDS, with a tight, deterministic
distribution (PDF has no heavy tail) versus UDS's wide scatter from OS
scheduling and kernel buffer locking — see the criterion reports in
`server_benchmarks/` for the raw PDF and regression plots.

To reproduce:

```bash
cargo bench
```

`deploy_bench.sh` provisions a clean Debian 12 host (build tools, gnuplot,
rustup) and runs the same benchmark; HTML output lands in
`target/criterion/report/index.html`.
