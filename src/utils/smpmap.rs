pub struct ChunkMeta {
    chunk_x: Int,
    chunk_y: Int,
    primary_bit_mask: u16
}

pub struct ChunkData {
}

pub struct ChunkBulkArray {
    chunk_meta: Vec<ChunkMeta>,
    chunks_data: Vec<ChunkData>
}