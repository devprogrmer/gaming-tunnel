use bytes::{Buf, BufMut, BytesMut};

pub const OUTER_HDR_LEN: usize = 8;
pub const INNER_HDR_LEN: usize = 4;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PktType { Data = 0, Parity = 1, Control = 2 }

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug)]
    pub struct Flags: u8 {
        const OBFUSCATED = 0b0001;
        const FEC_ON     = 0b0010;
        const REVERSE    = 0b0100;
    }
}

#[derive(Debug, Clone)]
pub struct OuterHeader {
    pub version: u8,       // 2 bits
    pub ptype: PktType,    // 2 bits
    pub flags: Flags,      // 4 bits
    pub block_id: u16,
    pub shard_index: u8,
    pub k: u8,             // data shards
    pub n: u8,             // total shards
    pub payload_len: u16,
}

impl OuterHeader {
    pub fn encode(&self, buf: &mut BytesMut) {
        let b0 = ((self.version & 0x03) << 6)
            | (((self.ptype as u8) & 0x03) << 4)
            | (self.flags.bits() & 0x0F);
        buf.put_u8(b0);
        buf.put_u16(self.block_id);
        buf.put_u8(self.shard_index);
        buf.put_u8(self.k);
        buf.put_u8(self.n);
        buf.put_u16(self.payload_len);
    }

    pub fn decode(buf: &mut impl Buf) -> Option<Self> {
        if buf.remaining() < OUTER_HDR_LEN { return None; }
        let b0 = buf.get_u8();
        let ptype = match (b0 >> 4) & 0x03 {
            0 => PktType::Data, 1 => PktType::Parity, _ => PktType::Control,
        };
        Some(Self {
            version: (b0 >> 6) & 0x03,
            ptype,
            flags: Flags::from_bits_truncate(b0 & 0x0F),
            block_id: buf.get_u16(),
            shard_index: buf.get_u8(),
            k: buf.get_u8(),
            n: buf.get_u8(),
            payload_len: buf.get_u16(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct InnerHeader { pub stream_id: u16, pub seq: u16 }

impl InnerHeader {
    pub fn encode(&self, buf: &mut BytesMut) {
        buf.put_u16(self.stream_id);
        buf.put_u16(self.seq);
    }
    pub fn decode(buf: &mut impl Buf) -> Option<Self> {
        if buf.remaining() < INNER_HDR_LEN { return None; }
        Some(Self { stream_id: buf.get_u16(), seq: buf.get_u16() })
    }
}
