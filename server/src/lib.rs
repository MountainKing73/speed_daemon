use futures::sink::SinkExt;
use log::{debug, error, info};
use shared::network::Camera;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{self, Sender};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time;
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, FramedWrite};

mod manager;

use shared::{
    messages::{MessageDecoder, MessageEncoder, MessageType},
    network::Ticket,
};

use crate::manager::{ManagerCommand, manager};

pub async fn run(addr: &str) {
    let listener = TcpListener::bind(addr).await.unwrap();
    info!("Listening on port 8080");

    let (manager_tx, manager_rx) = mpsc::channel::<ManagerCommand>(100);

    tokio::spawn(async move {
        manager(manager_rx).await;
    });

    loop {
        let (socket, _) = listener.accept().await.unwrap();
        debug!("Starting connection");
        let manager_tx = manager_tx.clone();
        tokio::spawn(async move {
            handle_client(socket, manager_tx).await;
        });
    }
}

#[derive(PartialEq)]
enum ClientType {
    Camera,
    Dispatcher,
    Unknown,
}

async fn send_heartbeat(t: u64, tx: Sender<String>) {
    println!("Setting up heartbeat time in nanoseconds: {}", t);
    let mut interval = time::interval(time::Duration::from_nanos(t));

    loop {
        interval.tick().await;
        println!("Send heartbeat");
        let _ = tx.send(String::from("Now")).await;
    }
}

async fn handle_client(mut stream: TcpStream, manager_tx: Sender<ManagerCommand>) {
    // Split the stream
    let (read, write) = stream.split();

    let encoder = MessageEncoder {};
    let mut writer = FramedWrite::new(write, encoder);
    let decoder = MessageDecoder {};
    let mut reader = FramedRead::new(read, decoder);

    let mut client_type: ClientType = ClientType::Unknown;
    let mut dispatcher_roads: Vec<u16> = vec![];
    let mut camera: Option<Camera> = None;

    let (heartbeat_tx, mut heartbeat_rx) = mpsc::channel::<String>(100);
    let mut heartbeat_task: Option<JoinHandle<_>> = None;

    while client_type == ClientType::Unknown {
        tokio::select! {
            Some(_) = heartbeat_rx.recv() => {
                println!("Got heartbeak signal");
                    let _=  writer.send(MessageType::HeartBeat).await;
            }
            result = reader.next() => match result {
            Some(Ok(msg)) => match msg {
                MessageType::IAmDispatcher(v) => {
                    client_type = ClientType::Dispatcher;
                    dispatcher_roads = v;
                }
                MessageType::IAmCamera(c) => {
                    client_type = ClientType::Camera;
                    camera = Some(c);
                }
                MessageType::WantHeartbeat(i) => {
                    let tx = heartbeat_tx.clone();
                    heartbeat_task = Some(tokio::spawn(async move {
                        send_heartbeat(i as u64 * 100000000, tx).await;
                    }));
                }
                _ => {
                    let _ = writer
                        .send(MessageType::Error(String::from(
                            "Type must be set before sending other messages",
                        )))
                        .await;
                }
            },
            Some(Err(e)) => error!("Error reading message {}", e),
            None => break,
        }
        }
    }

    if client_type == ClientType::Dispatcher {
        let (dispatcher_tx, mut dispatcher_rx) = mpsc::channel::<Ticket>(100);
        let (id_tx, id_rx) = oneshot::channel::<u16>();
        let _ = manager_tx
            .send(ManagerCommand::DispatcherJoin(
                dispatcher_tx,
                dispatcher_roads,
                id_tx,
            ))
            .await;

        let dispatcher_id = match id_rx.await {
            Ok(id) => id,
            Err(_) => panic!("Error joining dispatcher"),
        };

        loop {
            tokio::select! {
                Some(ticket) = dispatcher_rx.recv() => {
                    debug!("Received ticket {:?}", ticket);
                    let _ = writer.send(MessageType::Ticket(ticket)).await;
                }
                Some(_) = heartbeat_rx.recv() => {
                    println!("Got heartbeak signal");
                    let _=  writer.send(MessageType::HeartBeat).await;
                }
                result = reader.next() => match result {
                    Some(Ok(msg)) =>{
                        match msg {
                            MessageType::WantHeartbeat(i) => {
                                debug!("Received heartbeat request for interval {}", i);
                          let tx = heartbeat_tx.clone();
                          heartbeat_task = Some(tokio::spawn(async move {
                                send_heartbeat(i as u64 * 100000000, tx).await;
                          }));
                            }
                            _ => {
                                let _ = writer.send(MessageType::Error(String::from("Invalid message for Dispatcher"))).await;
                            }
                        }
                    }
                    Some(Err(e)) =>  error!("Error reading message {}", e),
                    None => {
                        let _ = manager_tx.send(ManagerCommand::Disconnect(dispatcher_id)).await;
                        break;
                    }
                }
            }
        }
    } else if client_type == ClientType::Camera {
        // Send road info to the manager
        let _ = manager_tx
            .send(ManagerCommand::Camera(
                camera.unwrap().road,
                camera.unwrap().limit,
            ))
            .await;
        loop {
            tokio::select! {
                      Some(_) = heartbeat_rx.recv() => {
                          let _=  writer.send(MessageType::HeartBeat).await;
                      }
                result = reader.next() => match result {
                    Some(Ok(msg)) =>{
                        match msg {
                            MessageType::Plate(plate, timestamp) => {
                                debug!("Received plate observation: {}, {}", plate, timestamp);
                                let _ = manager_tx.send(ManagerCommand::Plate(plate, camera.unwrap().road, camera.unwrap().mile, timestamp)).await;
                            }
                            MessageType::WantHeartbeat(i) => {
                                debug!("Received heartbeat request for interval {}", i);
                          let tx = heartbeat_tx.clone();
                          heartbeat_task = Some(tokio::spawn(async move {
                                send_heartbeat(i as u64 * 100000000, tx).await;
                          }));
                            }
                            _ => {
                                let _ = writer.send(MessageType::Error(String::from("Invalid message for Camera"))).await;
                            }
                        }
                    }
                    Some(Err(e)) =>  error!("Error reading message {}", e),
                    None => {
                              let _ = writer.close().await;
                        break;
                    }
                }
            }
        }
    }
    if let Some(t) = heartbeat_task {
        t.abort();
    }
}
