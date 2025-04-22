use log::debug;
use std::collections::HashMap;
use std::mem;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::oneshot;

use shared::network::Ticket;

pub struct Dispatcher {
    pub tx: Sender<Ticket>,
    pub roads: Vec<u16>,
}

pub struct Observation {
    pub road: u16,
    pub mile: u16,
    pub timestamp: u32,
}

pub enum ManagerCommand {
    DispatcherJoin(Sender<Ticket>, Vec<u16>, oneshot::Sender<u16>), // Channel to send tickets and list of roads plus one to return id
    Plate(String, u16, u16, u32), // Plate number, Road, mile marker and timestamp observation
    Camera(u16, u16),             // Road and speed limit
    Disconnect(u16),              // dispatcher id
}

enum TicketMgrCommand {
    DispatcherJoin(Sender<Ticket>, Vec<u16>, oneshot::Sender<u16>), // Channel to send tickets and list of roads plus one to return id
    Ticket(Ticket),
    Disconnect(u16), // dispatcher id
}

async fn ticket_manager(mut rx: Receiver<TicketMgrCommand>) {
    let mut dispatcher_id = 0;
    let mut dispatchers: HashMap<u16, Dispatcher> = HashMap::new();
    let mut ticket_history: Vec<(String, u32)> = vec![];
    let mut ticket_queue: HashMap<u16, Vec<Ticket>> = HashMap::new();

    while let Some(command) = rx.recv().await {
        match command {
            TicketMgrCommand::DispatcherJoin(dispatcher_tx, roads, id_tx) => {
                debug!("Adding new dispatcher id: {}", dispatcher_id);
                dispatchers.insert(
                    dispatcher_id,
                    Dispatcher {
                        tx: dispatcher_tx.clone(),
                        roads: roads.clone(),
                    },
                );
                let _ = id_tx.send(dispatcher_id);
                dispatcher_id += 1;
                // check if there is a ticket queue for this road and send if needed
                for road in roads {
                    let queue = ticket_queue.remove(&road);
                    if queue.is_some() {
                        for ticket in queue.unwrap() {
                            let _ = dispatcher_tx.send(ticket).await;
                        }
                    }
                }
            }
            TicketMgrCommand::Ticket(ticket) => {
                // 1. get days that the ticket covers - done
                // 2. check if car has been ticketed for each day - done
                // 3. send ticket to dispatcher for each day that hasn't been ticketed
                // 4. if no dispatcher, add to queue
                // 5. add ticket to history
                let day1 = (ticket.timestamp1 as f64 / 86400.0).floor() as u32;
                let day2 = (ticket.timestamp2 as f64 / 86400.0).floor() as u32;
                for day in day1..day2 + 1 {
                    if !ticket_history.contains(&(ticket.plate.clone(), day)) {
                        println!("Send ticket");
                        let plate = ticket.plate.clone();
                        let mut sent = false;
                        for dispatcher in dispatchers.values() {
                            if dispatcher.roads.contains(&ticket.road) {
                                debug!("Found dispatcher to send ticket");
                                let _ = dispatcher.tx.send(ticket.clone()).await;
                                sent = true;
                                break;
                            }
                        }
                        if !sent {
                            debug!("adding ticket to queue to send later");
                            let queue = ticket_queue.entry(ticket.road).or_default();
                            queue.push(ticket.clone());
                        }
                        ticket_history.push((plate, day));
                    }
                }
            }
            TicketMgrCommand::Disconnect(id) => {
                dispatchers.remove(&id);
            }
        }
    }
}

pub async fn manager(mut rx: Receiver<ManagerCommand>) {
    let mut cars: HashMap<String, Vec<Observation>> = HashMap::new();
    let mut roads: HashMap<u16, u16> = HashMap::new();

    let (ticket_mgr_tx, ticket_mgr_rx) = mpsc::channel::<TicketMgrCommand>(100);

    tokio::spawn(async move { ticket_manager(ticket_mgr_rx).await });

    while let Some(command) = rx.recv().await {
        match command {
            ManagerCommand::DispatcherJoin(dispatcher_tx, roads, id_tx) => {
                let _ = ticket_mgr_tx
                    .send(TicketMgrCommand::DispatcherJoin(
                        dispatcher_tx,
                        roads,
                        id_tx,
                    ))
                    .await;
            }
            ManagerCommand::Camera(road, limit) => {
                debug!("Add road {} with speed limit {}", road, limit);
                roads.insert(road, limit);
            }
            ManagerCommand::Plate(plate, road, mile, timestamp) => {
                debug!(
                    "Received plate observation: Plate: {} road: {} mile: {} timestamp: {}",
                    plate, road, mile, timestamp
                );
                // Check for ticket
                // Look at previous observations, if any are over speed limit, create ticket
                let observations = cars.get(&plate);
                if observations.is_some() {
                    for observation in observations.unwrap() {
                        if road == observation.road {
                            let limit = roads.get(&road).unwrap();
                            let elapsed_time =
                                timestamp.abs_diff(observation.timestamp) as f32 / 3600.0;
                            let speed = mile.abs_diff(observation.mile) as f32 / elapsed_time;
                            if speed > *limit as f32 {
                                let mut mile1 = observation.mile;
                                let mut timestamp1 = observation.timestamp;
                                let mut mile2 = mile;
                                let mut timestamp2 = timestamp;

                                // order the observations by timestamp
                                if timestamp1 > timestamp2 {
                                    mem::swap(&mut timestamp1, &mut timestamp2);
                                    mem::swap(&mut mile1, &mut mile2);
                                }
                                debug!("Found speeding car");
                                let _ = ticket_mgr_tx
                                    .send(TicketMgrCommand::Ticket(Ticket {
                                        plate: plate.clone(),
                                        road,
                                        mile1,
                                        timestamp1,
                                        mile2,
                                        timestamp2,
                                        speed: (speed * 100.0) as u16,
                                    }))
                                    .await;
                            }
                        }
                    }
                }
                // Add observation to cars
                let observations = cars.entry(plate).or_default();
                observations.push(Observation {
                    road,
                    mile,
                    timestamp,
                });
            }
            ManagerCommand::Disconnect(id) => {
                debug!("Dispatcher {} disconnected", id);
                let _ = ticket_mgr_tx.send(TicketMgrCommand::Disconnect(id)).await;
            }
        }
    }
}
