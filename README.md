# Xfeatures IPC 🚀

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)](https://github.com/xfeatures/xfeatures-ipc)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.80%2B-orange.svg)](https://www.rust-lang.org/)

**Xfeatures IPC** is a blisteringly fast, lock-free, Zero-Copy Single-Producer Single-Consumer (SPSC) ring buffer written in Rust. It facilitates ultra-low latency inter-process communication (IPC) via memory-mapped files (`mmap`), entirely bypassing OS syscall overhead during data transmission.

## Overview

Designed for high-frequency trading platforms, real-time analytics, and hyper-optimized microservice ecosystems, Xfeatures IPC guarantees that once the initial shared memory mapping is established, no further kernel transitions are made.

- **Zero-Copy**: Data is written directly into shared memory.
- **Lock-Free SPSC**: No Mutexes, no RwLocks, no Spinlocks blocking the critical path.
- **Fail-Fast**: Overflow policies immediately return `TryPushError` instead of busy-waiting.
- **Generic Typization**: Trivial, safe casting of any `T: Copy` structure into the ring buffer slots.

## ⚡ Performance

Server-grade benchmarks on Debian 12 (Linux) demonstrate industry-leading throughput and latency metrics.

- **Latency**: **8.9 ns** per message transmission.
- **Throughput**: **~112,000,000 msg/sec** (112M msg/sec).

### Unix Domain Sockets vs Xfeatures IPC
When benchmarked head-to-head against native Unix Domain Sockets (UDS) doing non-blocking I/O, Xfeatures IPC is roughly **57x faster**.
* Unix Domain Sockets: ~511 ns / msg
* Xfeatures IPC: ~8.9 ns / msg

<div align="center">
  <img src="./server_benchmarks/ipc_push_pop/report/pdf_small.svg" alt="Xfeatures IPC Latency" width="45%"/>
  <img src="./server_benchmarks/uds_push_pop/report/pdf_small.svg" alt="UDS Latency" width="45%"/>
  <br/>
  <i>Criterion.rs Latency Distribution (PDF): Xfeatures IPC (left) vs Unix Domain Sockets (right)</i>
</div>

### Jitter & Latency Correlation
Beyond simple averages, Xfeatures IPC exhibits drastically lower jitter (variance) compared to kernel-mediated sockets. The graphs below demonstrate the linear regression (correlation of time versus the number of iterations) and execution scatter.

<div align="center">
  <img src="./server_benchmarks/ipc_push_pop/report/regression_small.svg" alt="Xfeatures IPC Regression" width="45%"/>
  <img src="./server_benchmarks/uds_push_pop/report/regression_small.svg" alt="UDS Regression" width="45%"/>
  <br/>
  <i>Linear Regression & Scatter: Xfeatures IPC (left) vs Unix Domain Sockets (right)</i>
</div>

**How to interpret these graphs:**
- **Probability Density (PDF)**: The top graphs show that Xfeatures IPC latency tightly clumps around ~8.9 ns without a heavy tail (outliers). In contrast, UDS has a wide, unpredictable tail stretching into microseconds due to OS scheduling.
- **Regression (Time vs Iterations)**: The bottom graphs show execution time scaling with iterations. The left graph (Xfeatures) reveals a perfectly tight, concentrated linear correlation, proving the lock-free math is deterministic and rock-solid regardless of load. The right graph (UDS) displays significant vertical scatter and noise, representing the jitter introduced by OS context switches, kernel buffer locking, and system call overhead.

*(Raw criterion benchmark HTML reports are available in the `server_benchmarks` directory).*

## 🧠 Architecture Highlights

- **Cache-Line Padding (`#[repr(align(64))]`)**: The Producer's `head` index and Consumer's `tail` index are strictly aligned to 64-byte boundaries. This completely eliminates CPU false-sharing invalidations across L1/L2 caches.
- **Strict Memory Ordering**: We utilize highly optimized `Acquire`/`Release` atomic fences rather than heavyweight `SeqCst`. This ensures memory visibility guarantees with minimal CPU pipeline stalls.
- **Local Index Caching**: The Producer caches the Consumer's `tail` and the Consumer caches the Producer's `head`. The actual atomic load across the memory bus only occurs when the local cache indicates the buffer is empty/full, drastically reducing memory bus traffic.
- **Unaligned Memory Safety**: Generic `T: Copy` types are safely serialized via `ptr::write_unaligned` and `ptr::read_unaligned`, preventing SIGBUS exceptions and undefined behavior on ARM and strict-alignment architectures.

## Quick Start

Add `xfeatures-ipc` to your `Cargo.toml`.

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
    // Initialize both halves of the IPC ring buffer.
    // (Use build_producer() and build_consumer() for multi-process setups).
    let (mut prod, mut cons) = IpcBuilder::<SensorData>::new("sensor_stream")
        .capacity(1024) // Must be a power of two
        .build()
        .unwrap();

    // Consumer Thread
    let consumer_thread = thread::spawn(move || {
        while let Some(data) = cons.pop() {
            println!("Received: {:.1}C", data.temperature);
            break;
        }
    });

    // Producer Thread
    let data = SensorData {
        timestamp_ms: 1600000000,
        temperature: 22.5,
        humidity: 45.0,
    };
    
    // Fail-fast push: Returns Err if the buffer is full
    prod.push(data).expect("Buffer full");

    consumer_thread.join().unwrap();
}
```
