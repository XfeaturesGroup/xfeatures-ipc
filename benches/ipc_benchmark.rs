use criterion::{black_box, criterion_group, criterion_main, Criterion};
use xfeatures_ipc::IpcBuilder;
use std::mem;

#[derive(Copy, Clone)]
#[repr(C)]
struct BenchMessage {
    data: [u8; 256],
}

fn bench_throughput(c: &mut Criterion) {
    let (mut prod, mut cons) = IpcBuilder::<BenchMessage>::new("benchmark")
        .capacity(1024)
        .build()
        .unwrap();

    let data_in = BenchMessage { data: [0u8; 256] };

    c.bench_function("ipc_push_pop", |b| {
        b.iter(|| {
            while prod.push(black_box(data_in)).is_err() {
                std::hint::spin_loop();
            }
            while cons.pop().is_none() {
                std::hint::spin_loop();
            }
        })
    });
}

#[cfg(unix)]
fn bench_unix_socket(c: &mut Criterion) {
    use std::os::unix::net::UnixStream;
    use std::io::{Read, Write};

    // Create an anonymous pair of connected Unix sockets for equivalent SPSC testing
    let (mut sock_a, mut sock_b) = UnixStream::pair().unwrap();
    sock_a.set_nonblocking(true).unwrap();
    sock_b.set_nonblocking(true).unwrap();

    let data_in = BenchMessage { data: [0u8; 256] };
    let mut data_out = BenchMessage { data: [0u8; 256] };
    let item_size = mem::size_of::<BenchMessage>();

    c.bench_function("uds_push_pop", |b| {
        b.iter(|| {
            // Write to socket A
            let in_bytes = unsafe {
                std::slice::from_raw_parts(&data_in as *const _ as *const u8, item_size)
            };
            
            while let Err(ref e) = sock_a.write_all(black_box(in_bytes)) {
                if e.kind() != std::io::ErrorKind::WouldBlock {
                    panic!("UDS write failed: {}", e);
                }
                std::hint::spin_loop();
            }

            // Read from socket B
            let mut read_len = 0;
            let out_bytes = unsafe {
                std::slice::from_raw_parts_mut(&mut data_out as *mut _ as *mut u8, item_size)
            };
            
            while read_len < item_size {
                match sock_b.read(&mut out_bytes[read_len..]) {
                    Ok(0) => panic!("UDS closed"),
                    Ok(n) => read_len += n,
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::hint::spin_loop();
                    }
                    Err(e) => panic!("UDS read failed: {}", e),
                }
            }
        })
    });
}

#[cfg(unix)]
criterion_group!(benches, bench_throughput, bench_unix_socket);

#[cfg(not(unix))]
criterion_group!(benches, bench_throughput);

criterion_main!(benches);
