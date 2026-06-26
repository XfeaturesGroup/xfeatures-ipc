use std::env;
use std::process::Command;
use std::thread;
use xfeatures_ipc::IpcBuilder;

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(C)]
struct Message {
    id: u64,
    payload: [u8; 64],
}

#[test]
fn test_multiprocess_integration() {
    // Check if we are running as the child process (Consumer).
    if env::args().any(|arg| arg == "--consumer") {
        run_consumer();
        return; // run_consumer() terminates the process, this is unreachable
    }

    // 1. Producer creates the shared memory segment using the builder.
    let (mut prod, _cons) = IpcBuilder::<Message>::new("integration_test")
        .capacity(256)
        .build()
        .unwrap();

    // 2. Spawn the child process using the current test executable.
    let exe = env::current_exe().unwrap();
    let mut child = Command::new(exe)
        // Instruct libtest to run exactly this test function
        .arg("test_multiprocess_integration")
        .arg("--exact")
        // Pass arguments strictly to the test logic (ignored by libtest harness)
        .arg("--")
        .arg("--consumer")
        .spawn()
        .unwrap();

    // 3. Producer sends 1000 messages.
    for i in 0..1000 {
        let mut msg = Message {
            id: i as u64,
            payload: [0u8; 64],
        };
        msg.payload[0] = (i % 256) as u8;
        msg.payload[63] = 42; // Arbitrary marker to verify slot integrity
        
        while prod.push(msg).is_err() {
            // BufferFull: yield execution to allow the Consumer process to run
            thread::yield_now();
        }
    }

    // 4. Wait for the Consumer child process and verify success.
    let status = child.wait().unwrap();
    assert!(
        status.success(),
        "Child process (Consumer) did not exit successfully. Status: {}",
        status
    );
}

fn run_consumer() {
    // Connect to the shared memory using the builder as a consumer.
    let mut cons = IpcBuilder::<Message>::new("integration_test")
        .build_consumer()
        .expect("Failed to init consumer");

    for i in 0..1000 {
        let msg = loop {
            if let Some(m) = cons.pop() {
                break m;
            }
            // Buffer empty: yield execution to allow the Producer process to run
            thread::yield_now();
        };
        
        // Verify payload integrity
        assert_eq!(msg.id, i as u64, "Data corruption: incorrect sequence number");
        assert_eq!(msg.payload[0], (i % 256) as u8, "Data corruption: invalid payload byte");
        assert_eq!(msg.payload[63], 42, "Data corruption: missing magic marker");
    }
    
    // Terminate successfully immediately. This prevents the libtest harness
    // from attempting to run other tests in the child process context.
    std::process::exit(0);
}
