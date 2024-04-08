use std::{fs, env};
use std::sync::Arc;
use std::time::Duration;

use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::net::tcp::{OwnedWriteHalf, OwnedReadHalf};
use tokio::sync::broadcast::Sender;
use tokio::sync::{broadcast, Mutex};
use utils::packets::clientbound::{ClientboundDisconnectPacket, ClientboundMapChunkBulkPacket};
use utils::packets::serialization::{Int};
use utils::packets::{serialization, Packet};
use utils::stream_reader;
use uuid::Uuid;


mod utils;
use crate::utils::other::State;
use crate::utils::packets::clientbound::{ClientboundLoginSuccesPacket, ClientboundPingResponsePacket, ClientboundJoinGamePacket, ClientboundPluginMessagePacket, ClientboundStatusResponsePacket, ClientboundKeepAlivePacket, ClientboundSpawnEnityPacket, ClientboundSpawnPositionPacket};
use crate::utils::packets::serialization::Serializable;
use crate::utils::packets::serverbound::{ServerboundHandshakePacket, ServerboundStatusRequestPacket, ServerboundLoginStartPacket, ServerboundPingRequestPacket};
use crate::utils::smpmap::{ChunkBulkArray, ChunkData, ChunkMeta, ChunkSection, Position};

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
    env::set_var("RUST_BACKTRACE", "1");


    let server = Arc::new(Mutex::new(Server::new()));

    let addr = "127.0.0.1:25565";
    let listener = TcpListener::bind(&addr).await.unwrap();

    let (sender, _) = broadcast::channel::<Message>(5);

    let mut receiver = sender.subscribe();

    let thread_shared_server = server.clone();
    tokio::spawn(async move {
        loop {
            match receiver.recv().await {
                Ok(message) => {
                    println!("----------------------");
                    println!("Received: {:?}", message.0);
                    let _ = thread_shared_server.lock().await.handle_raw_packet(message.0, message.1).await;
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

    let thread_shared_server = server.clone();
    tokio::spawn(async move {
        let interval_seconds = 20; // Adjust interval as needed
        let mut interval = tokio::time::interval(Duration::from_secs(interval_seconds));

        loop {
            interval.tick().await;

            // Lock the server to access its connections
            let mut server = thread_shared_server.lock().await;

            // Iterate over connections
            for mut conn in &mut server.connections {
                // Send ping packet
                conn.write(ClientboundKeepAlivePacket {
                    id:  1723,
                }.serialize().as_slice()).await.expect("TODO: panic message");
            }

            // Allow other tasks to acquire the lock
            drop(server);
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
                println!("Length: {:?}", l);
                let l = l as usize;
                match reader.read_exact(&mut buffer[0..l]).await {
                    Ok(_) => {
                        let data = &buffer[0..l];
                        println!("{:?}", data);
                        let _ = channel_sender.send(Message{
                            0: data.to_vec(), 1: id as usize
                        });
                    }
                    Err(err) => {
                        eprintln!("Error reading data: {:?}", err);
                        break;
                    }
                }
            }
            Err(e) => {
                eprintln!("Error reading byte: {:?}", e);
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
        let data = data[1..].to_vec();
        println!("Pid: {}", pid);
        if self.states[id] == State::HANDSHAKE {
            match pid {
                0 => self.handle_handshake_packet(id, ServerboundHandshakePacket::new(data)).await,
                _ => unimplemented!("Unknown packet: HANDSHAKE:{pid}!")
            }
        } else if self.states[id] == State::STATUS {
            match pid {
                0 => self.handle_status_request_packet(id, ServerboundStatusRequestPacket::new(data)).await,
                1 => self.handle_ping_request_packet(id, ServerboundPingRequestPacket::new(data)).await,
                _ => {
                    let npid = pid;
                    Server::write_packet_file(pid, data);
                    println!("Unknown packet: STATUS:{npid}!");
                },
            }
        } else if self.states[id] == State::LOGIN {
            match pid {
                0 => self.handle_start_login_packet(id, ServerboundLoginStartPacket::new(data)).await,
                _ => {
                    let npid = pid;
                    Server::write_packet_file(pid, data);
                    println!("Unknown packet: LOGIN:{npid}!");
                },
            }
        } else if self.states[id] == State::PLAY {
            match pid {
                0x00 => {} //Keep Alive response from client
                0x03 => {} //Flying is onGround packet [Movement packet]
                _ => {
                    let npid = pid;
                    Server::write_packet_file(pid, data);
                    println!("Unknown packet: PLAY:{npid}!");
                },
            }
        }
    }

    fn write_packet_file(pid: i32, data: Vec<u8>) {
        let vec1 = pid.serialize();

        // Helper function to convert u8 value to hexadecimal string
        fn u8_to_hex(value: u8) -> String {
            format!("0x{:02X} ", value)
        }

        // Write the contents of vec1 as hexadecimal digits to the file
        for &byte in &vec1 {
            print!("{}", u8_to_hex(byte));
        }

        // Write the contents of vec2 as hexadecimal digits to the file
        for &byte in &data {
            print!("{}", u8_to_hex(byte));
        }

        println!("Data written to file successfully.");
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
    async fn handle_status_request_packet(&mut self, id: usize, _: ServerboundStatusRequestPacket) {
        println!("Send status");
        let s = fs::read_to_string("status.txt").unwrap();
        let _ = self.connections[id].write(ClientboundStatusResponsePacket{
            json_string: s,
        }.serialize().as_slice()).await;
    }
    async fn handle_ping_request_packet(&mut self, id: usize, packet: ServerboundPingRequestPacket) {
        let _ = self.connections[id].write(ClientboundPingResponsePacket {
            payload: packet.paylaod,
        }.serialize().as_slice());
    }

    //Loginpacket handler
    async fn handle_start_login_packet(&mut self, id: usize, packet: ServerboundLoginStartPacket) {
        let _ = self.connections[id].write(ClientboundLoginSuccesPacket {
            uuid: Server::generate_offline_uuid(&packet.name),
            username: packet.name,
        }.serialize().as_slice()).await;
        self.states[id] = State::PLAY;
        let _ = self.connections[id].write(ClientboundJoinGamePacket {
            id: Int { value: id as i32 },
            gamemode: 1,
            dimension: 0,
            difficulty: 0,
            max_players: 255,
            level_type: "default".to_owned(),
            reduced_debug_info: false,
        }.serialize().as_slice()).await;
        let _ = self.connections[id].write(ClientboundPluginMessagePacket {
            channel: "MC|Brand".to_owned(),
            data: "rapid".to_owned(),
        }.serialize().as_slice());


        let mut array: [u8; 8192] = [0; 8192];
        array.iter_mut().enumerate().for_each(|(i, x)| *x = if i % 2 == 0 { 2 << 4 } else { 2 >> 4 });

        let _ = self.connections[id].write(ClientboundMapChunkBulkPacket {
            sky_light_sent: true,
            chunk_column_count: 1,
            chunks: ChunkBulkArray {
                chunks: Vec::from([
                    ChunkData {
                        sections: [
                            ChunkSection {
                                blocks: array,
                                blocks_light: [2; 2048],
                                sky_light: [2; 2048],
                            }; 16
                        ],
                        meta: ChunkMeta {
                            chunk_x: Int { value: 0 },
                            chunk_z: Int { value: 0 },
                            primary_bit_mask: u16::MAX,
                        },
                    }
                ])
            },
        }.serialize().as_slice()).await;
        let _ = self.connections[id].write(ClientboundSpawnEnityPacket{
            entity_id: 42,
            tipe: 25,
            x: Int{value: 8},
            y: Int{value: 60},
            z: Int{value: 8},
            pitch: 0,
            yaw: 0,
            head_yam: 0,
            velocity_x: 0,
            velocity_y: 0,
            velocity_z: 0,
            metadata: 127,
        }.serialize().as_slice()).await;

        Server::write_packet_file(0x0F, ClientboundSpawnEnityPacket{
            entity_id: 2,
            tipe: 8,
            x: Int{value: 5},
            y: Int{value: 260},
            z: Int{value: 5},
            pitch: 0,
            yaw: 0,
            head_yam: 0,
            velocity_x: 0,
            velocity_y: 0,
            velocity_z: 0,
            metadata: 127,
        }.serialize());

        let file_path = "generated0x38.bin";
        // Attempt to create or open the file
        let mut file = match File::create(file_path).await {
            Ok(file) => file,
            Err(err) => {
                eprintln!("Error creating file: {}", err);
                return;
            }
        };
    }

    //Other
    fn generate_offline_uuid(username: &String) -> Uuid {
        // Define the OfflinePlayer namespace UUID
        let namespace = Uuid::parse_str("6ba7b810-9dad-11d1-80b4-00c04fd430c8").unwrap();
    
        // Generate the UUID based on the namespace and username
        let offline_uuid = Uuid::new_v3(&namespace, username.as_bytes());
        //println!("{}", offline_uuid.to_string());
        offline_uuid
    }
    async fn disconnect_all(&mut self) {
        for conn in &mut self.connections {
            let _ = conn.write(ClientboundDisconnectPacket{
                reason: "{\"text\":\"This is a test!\"}".to_owned(),
            }.serialize().as_slice());
        }
    }
}