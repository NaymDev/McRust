use crate::utils::packets::serialization::Int;

pub struct ChunkMeta {
    pub chunk_x: Int,
    pub chunk_z: Int,
    pub primary_bit_mask: u16
}

pub struct ChunkData {
    pub sections: [ChunkSection; 16],
    pub meta: ChunkMeta
}

pub struct ChunkSection {
    pub blocks: [u8; 8192],
    pub blocks_light: [u8; 2048],
    pub sky_light: [u8; 2048]
}

pub struct ChunkBulkArray {
    pub chunks: Vec<ChunkData>
}