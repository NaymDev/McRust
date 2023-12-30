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
    use uuid::Uuid;
    use uuid::{Builder, Version};

    use super::serialization::Serializeable;
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


    packet!(0, ServerboundHandshakePacket{
        protocol_version: i32,
        server_address: String,
        server_port: u16,
        next_state: i32,
    });
    packet!(0, ServerboundStatusRequestPacket{});

    packet!(0, ServerboundLoginStartPacket{
        name: String,
        player_uuid: Uuid,
    });
    packet!(3, ServerboundLoginAcknowledgedPacket{});
}

pub(crate) mod clientbound {
    use uuid::Uuid;
    use uuid::{Builder, Version};
    use super::serialization::Serializeable;
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


    packet!(0, ClientboundStatusResponsePacket{
        json_string: String
    });
    packet!(2, ClientboundLoginSuccesPacket{
        uuid: Uuid,
        username: String,
        num_of_props: i32,
    });
}

pub mod serialization  {
    use uuid::Builder;
    pub trait Serializeable {
        fn serialize(&self) -> Vec<u8>;
    }
    
    impl Serializeable for i32 {
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
    
    impl Serializeable for String {
        fn serialize(&self) -> Vec<u8> {
            let mut data = (self.len() as i32).serialize();
            data.extend(self.as_bytes());
            data
        }
    }
    
    impl Serializeable for u16 {
        fn serialize(&self) -> Vec<u8> {
            self.to_le_bytes().to_vec()
        }
    }

    impl Serializeable for i64 {
        fn serialize(&self) -> Vec<u8> {
            self.to_le_bytes().to_vec()
        }
    }

    impl Serializeable for Uuid {
        fn serialize(&self) -> Vec<u8> {
            self.to_u128_le().to_le_bytes().to_vec()
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
            println!("Index: {}", $index);
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
            i64::from_le_bytes([$data[$index-1],$data[$index-1],
                                $data[$index-3],$data[$index-4],
                                $data[$index-5],$data[$index-6],
                                $data[$index-7],$data[$index-8],])
        }};
        ($data:expr, $index:expr, Uuid) => {{
            $index+=16;
            Builder::from_bytes([$data[$index-1],$data[$index-2],$data[$index-3],$data[$index-4],
                                $data[$index-5],$data[$index-6],$data[$index-7],$data[$index-8],
                                $data[$index-9],$data[$index-10],$data[$index-11],$data[$index-12],
                                $data[$index-13],$data[$index-14],$data[$index-15],$data[$index-16]])
                                .set_version(Version::Random)
                                .set_variant(uuid::Variant::RFC4122)
                                .as_uuid().to_owned()
        }}; 
    }

    pub(crate) use deserialize;
    use uuid::Uuid;
}