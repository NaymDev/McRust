use std::{fs, env};
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::net::tcp::{OwnedWriteHalf, OwnedReadHalf};
use tokio::sync::broadcast::Sender;
use tokio::sync::{broadcast, Mutex};
use utils::packets::clientbound::ClientboundLoginSuccesPacket;
use utils::packets::serverbound::{ServerboundHandshakePacket, ServerboundStatusRequestPacket, ServerboundLoginStartPacket, ServerboundLoginAcknowledgedPacket};
use utils::packets::{serialization, clientbound, Packet};
use utils::stream_reader;



mod utils;
use crate::utils::other::State;
use crate::utils::packets::clientbound::ClientboundStatusResponsePacket;

#[derive(Debug)]
struct Message(Vec<u8>, usize);
impl Clone for Message {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone())
    }
}

#[tokio::main]
async fn main() {
    //DEBUG
    let _ = env::set_current_dir("D:/OpenMcRust/runtime");


    let mut server = Arc::new(Mutex::new(Server::new()));

    let addr = "127.0.0.1:25565";
    let listener = TcpListener::bind(&addr).await.unwrap();

    let (sender, _) = broadcast::channel::<Message>(5);

    let mut receiver = sender.subscribe();

    let thread_shared_server = server.clone();
    tokio::spawn(async move {
        loop {
            match receiver.recv().await {
                Ok(message) => {
                    println!("Received: {:#?}", message.0);
                    let e = thread_shared_server.lock().await.handle_raw_packet(message.0, message.1).await;
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    println!("Lagged {} messages", skipped);
                }
                Err(broadcast::error::RecvError::Closed) => {
                    println!("Channel closed, exiting continuous reader");
                    break;
                }
            }
        }
    });

    loop {
        let (socket, _) = listener.accept().await.unwrap();

        // Spawn a separate task to handle the connection
        let (reader, writer) = socket.into_split();
        let id = server.lock().await.add_connection_writer(writer);
        tokio::spawn(handle_connection(reader, sender.clone(), id));
    }
}

async fn handle_connection(mut reader: OwnedReadHalf, channel_sender: Sender<Message>, id: u8) {
    loop {
        let mut buffer = vec![0u8; 1024];
        
        match stream_reader::read_varint(&mut reader).await {
            Ok(l) => {
                let l = l as usize;
                match reader.read_exact(&mut buffer[0..l]).await {
                    Ok(_) => {
                        let data = &buffer[0..l];
                        let _ = channel_sender.send(Message{
                            0: data.to_vec(), 1: id as usize
                        });
                    }
                    Err(err) => {
                        //eprintln!("Error reading data: {:?}", err);
                        break;
                    }
                }
            }
            Err(e) => {
                //eprintln!("Error reading byte: {:?}", e);
                println!("Connection closed");
                break;
            }
        }
    }
}



struct Server {
    connections: Vec<OwnedWriteHalf>,
    states: Vec<State>,
}

impl Server {
    fn new() -> Server {
        Server{
            connections: Vec::new(),
            states: Vec::new(),
        }
    }

    async fn handle_raw_packet(&mut self, data: Vec<u8>, id: usize) {
        let mut index = 0;
        let pid = serialization::deserialize!(data, index, i32);
        println!("Pid: {}", pid);
        if self.states[id] == State::HANDSHAKE {
            match pid {
                0 => self.handle_handshake_packet(id, ServerboundHandshakePacket::new(data)).await,
                _ => unimplemented!("Unknown packet!")
            }
        } else if self.states[id] == State::STATUS {
            match pid {
                0 => self.handle_status_request_packet(id, ServerboundStatusRequestPacket::new(data)).await,
                _ => unimplemented!("Unknown packet!")
            }
        } else if self.states[id] == State::LOGIN {
            match pid {
                0 => self.handle_start_login_packet(id, ServerboundLoginStartPacket::new(data)).await,
                3 => self.handle_login_acknowledged_packet(id, ServerboundLoginAcknowledgedPacket::new(data)).await,
                _ => unimplemented!("Unknown packet!"),
            }
        }
    }

    fn add_connection_writer(&mut self, writer: OwnedWriteHalf) -> u8 {
        self.connections.push(writer);
        self.states.push(State::HANDSHAKE);
        (self.connections.len() as u8)-1
    }

    async fn handle_handshake_packet(&mut self, id: usize, packet: ServerboundHandshakePacket) {
        match packet.next_state {
            1 => self.states[id] = State::STATUS,
            _ => self.states[id] = State::LOGIN,
        }
    }

    //Statuspacket handler
    async fn handle_status_request_packet(&mut self, id: usize, packet: ServerboundStatusRequestPacket) {
        println!("Send status");
        let s = fs::read_to_string("status.txt").unwrap();
        let _ = self.connections[id].write(ClientboundStatusResponsePacket{
            json_string: s,
        }.serialize().as_slice()).await;
    }

    //Loginpacket handler
    async fn handle_start_login_packet(&mut self, id: usize, packet: ServerboundLoginStartPacket) {
        let _ = self.connections[id].write(ClientboundLoginSuccesPacket{
            uuid: packet.player_uuid,
            username: packet.name,
            num_of_props: 0,
        }.serialize().as_slice()).await;
    }

    async fn handle_login_acknowledged_packet(&mut self, id: usize, packet: ServerboundLoginAcknowledgedPacket) {
        self.states[id] = State::CONFIGURATION;
    }
}