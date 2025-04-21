use futures::sink::SinkExt;
use log::{debug, error, info};
use std::io::{self, Write};
use tokio::net::TcpStream;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, FramedWrite};

use shared::messages::{MessageDecoder, MessageEncoder, MessageType};
use shared::network::Camera;

pub async fn run(connect: String) {
    let mut client = TcpStream::connect(connect.clone()).await.unwrap();
    info!("Connected to {}", connect);

    let (read, write) = client.split();
    let encoder = MessageEncoder {};
    let mut writer = FramedWrite::new(write, encoder);
    let decoder = MessageDecoder {};
    let mut reader = FramedRead::new(read, decoder);

    // Create channel to communicate with user
    let (client_tx, mut client_rx): (Sender<MessageType>, Receiver<MessageType>) =
        mpsc::channel(32);
    tokio::spawn(async move {
        handle_user(client_tx).await;
    });

    loop {
        tokio::select! {
            Some(m) = client_rx.recv() => {
                debug!("Send message {:?} to server", m);
                writer.send(m).await.expect("Error sending message");
            }
            result = reader.next() => match result {
                Some(Ok(msg)) => {
                    debug!("Received from server: {:?}", msg);
                    match msg {
                        MessageType::Error(e) => println!("Received error: {}", e),
                        MessageType::Ticket(t) => println!("Received ticket: {:?}", t),
                        MessageType::HeartBeat => println!("Received heartbeat"),
                        _ => unimplemented!("Received unimplemented message from server"),
                    }
                }
                Some(Err(e)) => {
                    error!("Error reading message {}", e);
                    break;
                }
                None => {
                    debug!("Disconnect");
                    std::process::exit(0);
                }
            }
        }
    }
}

async fn handle_user(tx: Sender<MessageType>) {
    loop {
        println!("Welcome to the Speed Daemon client.");
        println!("Pick an option: ");
        println!("  1. Set type to camera");
        println!("  2. Set type to dispatcher");
        println!("  3. Send plate");
        println!("  4. Send heartbeat request");
        println!("  5. Quit");

        let mut buffer = String::new();
        let stdin = io::stdin();
        stdin.read_line(&mut buffer).expect("Error reading input");

        match buffer.trim() {
            "1" => {
                let stdin = io::stdin();

                print!("What road are you on: ");
                let _ = io::stdout().flush();
                let mut buffer = String::new();
                stdin.read_line(&mut buffer).expect("Error reading input");
                let road: u16 = buffer.trim().parse().expect("Input not an integer");

                print!("What mile marker are you at: ");
                let _ = io::stdout().flush();
                buffer = String::new();
                stdin.read_line(&mut buffer).expect("Error reading input");
                let mile: u16 = buffer.trim().parse().expect("Input not an integer");

                print!("What is the speed limit: ");
                let _ = io::stdout().flush();
                buffer = String::new();
                stdin.read_line(&mut buffer).expect("Error reading input");
                let limit: u16 = buffer.trim().parse().expect("Input not an integer");

                _ = tx
                    .send(MessageType::IAmCamera(Camera { road, mile, limit }))
                    .await;
            }
            "2" => {
                let stdin = io::stdin();

                print!("How many roads are you responsible for: ");
                let _ = io::stdout().flush();
                let mut buffer = String::new();
                stdin.read_line(&mut buffer).expect("Error reading input");
                let count: u16 = buffer.trim().parse().expect("Input not an integer");

                let mut roads: Vec<u16> = vec![];
                for i in 0..count {
                    print!("Enter road number {}: ", i + 1);
                    let _ = io::stdout().flush();
                    buffer = String::new();
                    stdin.read_line(&mut buffer).expect("Error reading input");
                    let road: u16 = buffer.trim().parse().expect("Input not an integer");
                    roads.push(road);
                }

                _ = tx.send(MessageType::IAmDispatcher(roads)).await;
            }
            "3" => {
                let stdin = io::stdin();

                print!("What license plate did you observe: ");
                let _ = io::stdout().flush();
                let mut buffer = String::new();
                stdin.read_line(&mut buffer).expect("Error reading input");
                let plate = buffer.trim().to_string();

                print!("What time did you observe it: ");
                let _ = io::stdout().flush();
                buffer = String::new();
                stdin.read_line(&mut buffer).expect("Error reading input");
                let timestamp: u32 = buffer.trim().parse().expect("Input not an integer");

                _ = tx.send(MessageType::Plate(plate, timestamp)).await;
            }
            "4" => {
                let stdin = io::stdin();

                print!("How often do you want the heartbeat: ");
                let _ = io::stdout().flush();
                let mut buffer = String::new();
                stdin.read_line(&mut buffer).expect("Error reading input");
                let interval: u32 = buffer.trim().parse().expect("Input not an integer");

                _ = tx.send(MessageType::WantHeartbeat(interval)).await;
            }
            "5" => {
                debug!("Exiting client");
                std::process::exit(0);
            }
            value => println!("Option not implemented: {}", value),
        }
    }
}
