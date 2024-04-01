pub trait Packet {
    fn serialize(&self) -> Vec<u8>;
    fn deserialize(&mut self, data: Vec<u8>)
    where
        Self: Sized;
    
    fn new(data: Vec<u8>) -> Self
    where
        Self: Sized;
}


pub(crate) mod serverbound {
    use super::serialization::Serializable;
    use super::serialization::deserialize;

    // Define a macro to generate common serialization and deserialization code
    macro_rules! packet {
        ($id:expr, $name:ident { $($field:ident : $ty:tt),* $(,)? }) => {
            #[derive(Default)]
            pub struct $name {
                $(pub $field: $ty),*
            }

            impl crate::utils::packets::Packet for $name {
                fn serialize(&self) -> Vec<u8> {
                    let mut buffer: Vec<u8> = Vec::new();
                    $(
                        buffer.extend(&self.$field.serialize());
                    )*
                    let pid = $id.serialize();
                    let mut final_buf = ((buffer.len()+pid.len()) as i32).serialize();
                    final_buf.extend(pid);
                    final_buf.extend(buffer);
                    final_buf
                }

                fn deserialize(&mut self, data: Vec<u8>) {
                    let mut index = 0;
                    $(
                        self.$field = deserialize!(data, index, $ty);
                    )*
                }

                fn new(data: Vec<u8>) -> $name {
                    let mut p: $name = Default::default();
                    p.deserialize(data);
                    p
                }
            }
        };
    }

    //STATUS
    packet!(0, ServerboundHandshakePacket{
        protocol_version: i32,
        server_address: String,
        server_port: u16,
        next_state: i32,
    });
    packet!(0, ServerboundStatusRequestPacket{});
    packet!(1, ServerboundPingRequestPacket{
        paylaod: i64,
    });

    //LOGIN
    packet!(0, ServerboundLoginStartPacket{
        name: String,
    });
    packet!(3, ServerboundLoginAcknowledgedPacket{});

    //PLAY
    packet!(1, ServerboundChatMessagenPacket{
        message: String,
    });
    packet!(2, ServerboundUseEntityPacket{
        target: i32,
        _type: i32,
    });
    packet!(3, SerevrboundPlayerPacket{
        on_ground: bool,
    });
}

pub(crate) mod clientbound {
    use uuid::Uuid;

    use crate::utils::smpmap::ChunkBulkArray;

    use super::serialization::{Serializable, Int};
    use super::serialization::deserialize;
    // Define a macro to generate common serialization and deserialization code
    macro_rules! packet {
        ($id:expr, $name:ident { $($field:ident : $ty:ident),* $(,)? }) => {
            #[derive(Default)]
            pub struct $name {
                $(pub $field: $ty),*
            }

            impl crate::utils::packets::Packet for $name {
                fn serialize(&self) -> Vec<u8> {
                    let mut buffer: Vec<u8> = Vec::new();
                    $(
                        buffer.extend(&self.$field.serialize());
                    )*
                    let pid = $id.serialize();
                    let mut final_buf = ((buffer.len()+pid.len()) as i32).serialize();
                    final_buf.extend(pid);
                    final_buf.extend(buffer);
                    final_buf
                }

                fn deserialize(&mut self, data: Vec<u8>) {
                    let mut index = 0;
                    $(
                        self.$field = deserialize!(data, index, $ty);
                    )*
                }

                fn new(data: Vec<u8>) -> $name {
                    let mut p: $name = Default::default();
                    p.deserialize(data);
                    p
                }
            }
        };
    }

    //STATUS
    packet!(0, ClientboundStatusResponsePacket{
        json_string: String
    });
    packet!(1, ClientboundPingResponsePacket{
        payload: i64,
    });

    //LOGIN
    packet!(0, ClientboundDisconnectPacket{
        reason: String,
    });
    packet!(2, ClientboundLoginSuccesPacket{
        uuid: Uuid,
        username: String,
    });

    //PLAY
    packet!(0, ClientboundKeepAlivePacket{
        id: i32,
    });
    packet!(1, ClientboundJoinGamePacket{
        id: Int,
        gamemode: u8,
        dimension: i8,
        difficulty: u8,
        max_players: u8,
        level_type: String,
        reduced_debug_info: bool,
    });
    packet!(0x3F, ClientboundPluginMessagePacket{
        channel: String,
        data: String,
    });
    packet!(0x41, ClientboundDifficultyPacket{
        difficulty: u8,
    });
    packet!(0x26, ClientboundMapChunkBulkPacket{
        sky_light_sent: bool,
        chunk_column_count: i32,
        chunks: ChunkBulkArray,
    });
}

pub mod serialization  {
    #[derive(Default)]
    pub struct Int {
        pub value: i32,
    }
    impl Serializable for Int {
        fn serialize(&self) -> Vec<u8> {
            self.value.to_le_bytes().to_vec()
        }
    }


    pub trait Serializable {
        fn serialize(&self) -> Vec<u8>;
    }
    
    impl Serializable for i32 {
        fn serialize(&self) -> Vec<u8> {
            const SEGMENT_BITS: i32 = 0x7F;
            const CONTINUE_BIT: i32 = 0x80;
            
            let mut result = Vec::new();
            let mut val = self.to_owned();
            
            loop {
                if (val & !SEGMENT_BITS) == 0 {
                    result.push(val as u8);
                    return result;
                }
            
                result.push((val & SEGMENT_BITS | CONTINUE_BIT) as u8);
            
                // Note: >> is used for right shift in Rust
                val >>= 7;
            }
        }
    }
    
    impl Serializable for String {
        fn serialize(&self) -> Vec<u8> {
            let mut data = (self.chars().count() as i32).serialize();
            data.extend(self.as_bytes());
            data
        }
    }
    
    impl Serializable for u16 {
        fn serialize(&self) -> Vec<u8> {
            self.to_le_bytes().to_vec()
        }
    }

    impl Serializable for i64 {
        fn serialize(&self) -> Vec<u8> {
            self.to_be_bytes().to_vec()
        }
    }

    impl Serializable for Uuid {
        fn serialize(&self) -> Vec<u8> {
            self.to_string().serialize()
        }
    }

    impl Serializable for u8 {
        fn serialize(&self) -> Vec<u8> {
            self.to_le_bytes().to_vec()
        }
    }
    impl Serializable for i8 {
        fn serialize(&self) -> Vec<u8> {
            self.to_le_bytes().to_vec()
        }
    }
    
    impl Serializable for bool {
        fn serialize(&self) -> Vec<u8> {
            if *self {[0x01].to_vec()} else {[0x00].to_vec()}
        }
    }

    impl Serializable for ChunkBulkArray {
        fn serialize(&self) -> Vec<u8> {
            let mut data=Vec::new();
            for chunk in self.chunks {
                data.extend(chunk.meta.chunk_x.serialize());
                data.extend(chunk.meta.chunk_z.serialize());
                data.extend(chunk.meta.primary_bit_mask.serialize());
                const SECTION_COUNT: usize = usize::from(u16::count_ones(chunk.meta.primary_bit_mask));
                //let mut blocks: [u8; 8192*SECTION_COUNT];
                //let mut blocks_light: [u8; 2048*SECTION_COUNT];
                //let mut sky_light: [u8; 2048*SECTION_COUNT];

                let mut blocks = Vec::new();
                let mut blocks_light = Vec::new();
                let mut sky_light = Vec::new();
                for (i, section) in chunk.sections.iter().enumerate() {
                    if chunk.meta.primary_bit_mask & (1 << i) {
                        blocks.extend(section.blocks);
                        blocks_light.extend(section.blocks_light);
                        sky_light.extend(section.sky_light);
                    }
                }
                data.extend(blocks);
                data.extend(blocks_light);
                data.extend(sky_light);
                data.extend([0; 256])
            }
            data
        }
    }
    
    macro_rules! deserialize {
        ($data:expr, $index:expr, i32) => {{
            let mut result = 0;
            let mut shift = 0;
    
            loop {
                let byte = $data[$index];
                $index+=1;
                result |= ((byte & 0x7F) as u64) << shift;
                if (byte & 0x80) == 0 {
                    break;
                }
                shift += 7;
            }
            result as i32
        }};
        ($data:expr, $index:expr, String) => {{
            let len: i32 = deserialize!($data, $index, i32);
            if $index < usize::MAX {
                let offset = len as usize;
                let end_index = ($index).saturating_add(offset);
    
                // Get the subarray
                let subarray = &$data[$index..end_index];
                $index = end_index;
    
                // Convert the subarray to a string
                if let Ok(utf8_string) = std::str::from_utf8(subarray) {
                    utf8_string.to_owned()
                } else {
                    println!("Error: Invalid UTF-8 data in subarray");
                    "".to_owned()
                }
            } else {
                println!("Error: Index out of bounds");
                "".to_owned()
            }
        }};
        ($data:expr, $index:expr, u16) => {{
            $index+=2;
            u16::from_le_bytes([$data[$index-1],
                                $data[$index-2]])
        }};
        ($data:expr, $index:expr, i64) => {{
            $index+=8;
            i64::from_le_bytes([$data[$index-1],$data[$index-2],
                                $data[$index-3],$data[$index-4],
                                $data[$index-5],$data[$index-6],
                                $data[$index-7],$data[$index-8],])
        }};
        ($data:expr, $index:expr, Uuid) => {{
            Uuid::parse_str(&deserialize!($data, $index, String)).unwrap()
        }}; 
        ($data:expr, $index:expr, Int) => {{
            $index+=4;
            Int{
                value: i32::from_le_bytes([
                    $data[$index-1],$data[$index-2],
                    $data[$index-3],$data[$index-4],
                ])
            }
        }}; 
        ($data:expr, $index:expr, u8) => {{
            $index+=1;
            u8::from_le_bytes([$data[$index-1]])
        }};
        ($data:expr, $index:expr, i8) => {{
            $index+=1;
            i8::from_le_bytes([$data[$index-1]])
        }};
        ($data:expr, $index:expr, bool) => {{
            $index+=1;
            if $data[$index-1] == 0x01 {true} else {false}
        }};
        ($data:expr, $index:expr, ChunkBulkArray) => {{
            ChunkBulkArray::default()
        }};
    }

    pub(crate) use deserialize;
    use uuid::Uuid;

    use crate::utils::smpmap::{ChunkBulkArray, ChunkData};
}