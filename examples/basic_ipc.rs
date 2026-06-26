use xfeatures_ipc::IpcBuilder;
use std::thread;
use std::time::Duration;

#[derive(Copy, Clone, Debug)]
#[repr(C)]
struct SensorData {
    timestamp_ms: u64,
    temperature: f32,
    humidity: f32,
}

fn main() {
    println!("Initializing Xfeatures IPC Ring Buffer...");

    // 1. Create the IPC channel using the builder.
    // For this example, we build both halves in the same process,
    // but typically `build_producer()` and `build_consumer()` are used across processes.
    let (mut prod, mut cons) = IpcBuilder::<SensorData>::new("sensor_stream")
        .capacity(1024)
        .build()
        .expect("Failed to create IPC channel");

    // 2. Spawn a Consumer thread
    let consumer_thread = thread::spawn(move || {
        let mut received = 0;
        
        println!("[Consumer] Waiting for data...");
        while received < 5 {
            if let Some(data) = cons.pop() {
                println!(
                    "[Consumer] Received: Time: {}ms, Temp: {:.1}C, Hum: {:.1}%",
                    data.timestamp_ms, data.temperature, data.humidity
                );
                received += 1;
            } else {
                // Yield to prevent pegging the CPU to 100% in this simple example
                thread::yield_now();
            }
        }
        println!("[Consumer] Finished reading.");
    });

    // 3. Produce data in the main thread
    for i in 0..5 {
        let data = SensorData {
            timestamp_ms: 1600000000 + (i * 1000),
            temperature: 22.5 + (i as f32 * 0.1),
            humidity: 45.0 + (i as f32 * 0.5),
        };
        
        while prod.push(data).is_err() {
            // Buffer full, wait for consumer
            thread::yield_now();
        }
        
        println!("[Producer] Sent data packet {}", i + 1);
        thread::sleep(Duration::from_millis(100)); // Simulate work
    }

    // Wait for consumer to finish
    consumer_thread.join().unwrap();
    println!("IPC demo completed successfully.");
}
