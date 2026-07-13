use reed_solomon_erasure::galois_8::ReedSolomon;
use anyhow::Result;

pub struct FecCodec {
    rs: Option<ReedSolomon>,
    pub data: usize,
    pub parity: usize,
    pub shard_size: usize,
}

impl FecCodec {
    pub fn new(data: usize, parity: usize, shard_size: usize) -> Result<Self> {
        let rs = if data > 0 && parity > 0 {
            Some(ReedSolomon::new(data, parity)?)
        } else { None };
        Ok(Self { rs, data, parity, shard_size })
    }

    /// Encode `data` shards -> returns full set (data + parity), all padded to shard_size.
    pub fn encode_block(&self, mut shards: Vec<Vec<u8>>) -> Result<Vec<Vec<u8>>> {
        let Some(rs) = &self.rs else { return Ok(shards); };
        for s in shards.iter_mut() { s.resize(self.shard_size, 0); }
        for _ in 0..self.parity { shards.push(vec![0u8; self.shard_size]); }
        rs.encode(&mut shards)?;
        Ok(shards)
    }

    /// Reconstruct missing shards. `shards[i] == None` means lost.
    pub fn decode_block(&self, shards: &mut [Option<Vec<u8>>]) -> Result<()> {
        let Some(rs) = &self.rs else { return Ok(()); };
        rs.reconstruct_data(shards)?;
        Ok(())
    }
}
