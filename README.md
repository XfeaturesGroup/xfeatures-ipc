# Xfeatures IPC — zero-copy SPSC ring buffer

A lock-free, zero-copy Single-Producer Single-Consumer ring buffer for Rust.
Producer and consumer talk over a memory-mapped file (`mmap`); once the
initial mapping is established, no further syscalls happen on the hot path.

```
src/builder.rs            IpcBuilder — creates the mapping, splits producer/consumer
src/ring_buffer.rs        lock-free SPSC ring, Acquire/Release index handoff
src/layout.rs             shared-memory layout of the header + slots
src/mmap.rs               mmap wrapper around the backing file
examples/basic_ipc.rs     single-process producer/consumer demo
benches/ipc_benchmark.rs  criterion benchmark, Xfeatures IPC vs Unix Domain Sockets
server_benchmarks/        raw criterion HTML/SVG reports (ipc_push_pop, uds_push_pop)
deploy_bench.sh           provisions a clean Debian 12 box and runs the benchmark
```

## Quick start

1. Add `xfeatures-ipc` to `Cargo.toml`.
2. Build a producer/consumer pair and push/pop any `T: Copy` type:

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

3. Or run the fuller demo:

   ```bash
   cargo run --example basic_ipc
   ```

The snippet above builds both halves in the same process with `.build()`; across
separate processes, call `build_producer()` and `build_consumer()` against the
same name from each side instead.

## Benchmark

```bash
cargo bench
```

Runs the criterion suite comparing Xfeatures IPC against non-blocking Unix
Domain Sockets, measured on Debian 12. HTML output lands in
`target/criterion/report/index.html`. `deploy_bench.sh` provisions a bare
Debian 12 host (build tools, gnuplot, rustup) and runs the same command on
a remote box.

<div align="center">
  <img src="./server_benchmarks/ipc_push_pop/report/pdf_small.svg" alt="Xfeatures IPC Latency" width="45%"/>
  <img src="./server_benchmarks/uds_push_pop/report/pdf_small.svg" alt="UDS Latency" width="45%"/>
  <br/>
  <i>Criterion.rs latency distribution (PDF): Xfeatures IPC (left) vs Unix Domain Sockets (right)</i>
</div>

<div align="center">
  <img src="./server_benchmarks/ipc_push_pop/report/regression_small.svg" alt="Xfeatures IPC Regression" width="45%"/>
  <img src="./server_benchmarks/uds_push_pop/report/regression_small.svg" alt="UDS Regression" width="45%"/>
  <br/>
  <i>Linear regression & scatter: Xfeatures IPC (left) vs Unix Domain Sockets (right)</i>
</div>

Xfeatures IPC latency clumps tightly around 8.9 ns with no heavy tail; Unix
Domain Sockets show a wide, unpredictable tail stretching into microseconds
from OS scheduling. The regression plots tell the same story over iterations:
Xfeatures IPC scales along a tight, deterministic line, while UDS shows the
vertical scatter of kernel buffer locking and syscall overhead. Raw criterion
HTML reports are in `server_benchmarks/`.

## Results

| | Latency | Throughput |
|---|---|---|
| Xfeatures IPC | 8.9 ns/msg | ~112,000,000 msg/sec |
| Unix Domain Sockets | ~511 ns/msg | — |

Head-to-head, Xfeatures IPC is roughly 57x faster than UDS.

## Architecture

Cache-line padding (`#[repr(align(64))]`) keeps the producer's `head` and the
consumer's `tail` on separate 64-byte lines, so false-sharing can't invalidate
them across cores. Memory visibility uses `Acquire`/`Release` fences rather
than `SeqCst`, and each side caches the other's index locally — the atomic
load across the memory bus only happens once that local cache says
empty/full, which keeps bus traffic down. Generic `T: Copy` values are
(de)serialized with `ptr::write_unaligned`/`read_unaligned`, so the ring
buffer is sound on strict-alignment targets like ARM, not just x86. A full
buffer returns `TryPushError` immediately — fail-fast, no busy-waiting.
