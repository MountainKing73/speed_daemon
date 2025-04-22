use log::debug;
use std::net::TcpListener;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::sleep;

use server::run;

use std::sync::Once;
static INIT: Once = Once::new();

pub fn init_logger() {
    INIT.call_once(|| {
        env_logger::builder().is_test(true).init();
    });
}

fn get_free_address() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().to_string()
}

fn hex_to_bytes(hex: &str) -> Vec<u8> {
    hex::decode(hex).expect("Invalid hex string")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_heartbeat_request() {
    init_logger();
    let addr = get_free_address();

    let server_addr = addr.clone();
    tokio::spawn(async move {
        run(&server_addr).await;
    });

    sleep(Duration::from_millis(100)).await;

    let addr = addr.clone();
    let handle = tokio::spawn(async move {
        let mut stream = TcpStream::connect(&addr)
            .await
            .expect("Failed to connect to server");

        // Send IAmDispatcher message to server
        let bytes = hex_to_bytes("81010042");
        stream.write_all(&bytes).await.expect("Write failed");
        let messages = vec!["400000000a"];

        for message in &messages {
            let bytes = hex_to_bytes(message);
            stream.write_all(&bytes).await.expect("Write failed");

            sleep(Duration::from_millis(50)).await;

            let mut buf = vec![0u8; 1024];
            let n = stream.read(&mut buf).await.expect("Read failed");

            let response = &buf[..n];
            debug!("Response: {:?}", response);

            assert!(!response.is_empty(), "Empty response from server");
            assert_eq!(response, &hex_to_bytes("41"));
        }
    });

    handle.await.unwrap();
}
