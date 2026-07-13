use crate::config::{RunArgs, FecRatio};
use crate::protocol::*;
use crate::{fec::FecCodec, crypto::Obfuscator, pacing::Pacer, jitter::JitterBuffer};
use anyhow::Result;
use bytes::BytesMut;
use tokio::sync::Mutex;

pub struct Pipeline {
    fec: FecCodec,
    obfs: Option<Obfuscator>,
    pacer: Mutex<Pacer>,
    mtu: usize,
    mux_streams: u16,
    jitter_depth: usize,
    jitter_ms: u64,
    reverse: bool,
}

impl Pipeline {
    pub fn new(args: &RunArgs, fec: FecRatio) -> Result<Self> {
        let shard_size = args.mtu as usize + INNER_HDR_LEN;
        Ok(Self {
            fec: FecCodec::new(fec.data, fec.parity, shard_size)?,
            obfs: if args.obfs { Some(Obfuscator::new(&args.key)) } else { None },
            pacer: Mutex::new(Pacer::new(args.pace_mbps)),
            mtu: args.mtu as usize,
            mux_streams: args.mux,
            jitter_depth: args.jitter,
            jitter_ms: args.jitter_ms,
            reverse: args.reverse,
        })
    }

    // ---------- TX: TUN -> pipeline -> UDP ----------
    pub async fn tx_loop<R, W>(&self, mut tun_rd: R, mut sock_wr: W) -> Result<()>
    where R: TunRead, W: SockWrite {
        let mut block_id: u16 = 0;
        let mut seq: u16 = 0;
        let mut batch: Vec<Vec<u8>> = Vec::with_capacity(self.fec.data.max(1));

        loop {
            let ip_frame = tun_rd.read_frame().await?; // raw IP packet

            // 1. MUX: tag with stream_id + seq (inner header)
            let stream_id = crate::mux::select_stream(&ip_frame, self.mux_streams);
            let mut inner = BytesMut::with_capacity(INNER_HDR_LEN + ip_frame.len());
            InnerHeader { stream_id, seq }.encode(&mut inner);
            inner.extend_from_slice(&ip_frame);
            seq = seq.wrapping_add(1);

            batch.push(inner.to_vec());

            // 2. FEC: once we have k data shards, encode block + parity
            let need = if self.fec.data == 0 { 1 } else { self.fec.data };
            if batch.len() >= need {
                let shards = self.fec.encode_block(std::mem::take(&mut batch))?;
                self.emit_block(&mut sock_wr, block_id, &shards).await?;
                block_id = block_id.wrapping_add(1);
            }
        }
    }

    async fn emit_block<W: SockWrite>(&self, sock_wr: &mut W, block_id: u16, shards: &[Vec<u8>]) -> Result<()> {
        let k = self.fec.data as u8;
        let n = self.fec.total().max(1) as u8;
        for (idx, shard) in shards.iter().enumerate() {
            let ptype = if (idx as usize) < self.fec.data { PktType::Data } else { PktType::Parity };
            let mut flags = Flags::empty();
            if self.fec.data > 0 { flags |= Flags::FEC_ON; }
            if self.obfs.is_some() { flags |= Flags::OBFUSCATED; }
            if self.reverse { flags |= Flags::REVERSE; }

            let mut pkt = BytesMut::with_capacity(OUTER_HDR_LEN + shard.len());
            OuterHeader {
                version: 1, ptype, flags,
                block_id, shard_index: idx as u8,
                k, n, payload_len: shard.len() as u16,
            }.encode(&mut pkt);
            pkt.extend_from_slice(shard);

            // 3. Obfuscate whole datagram body (after outer header for keystream)
            if let Some(o) = &self.obfs { o.apply(&mut pkt[OUTER_HDR_LEN..], block_id); }

            // 4. Pace, then send
            self.pacer.lock().await.wait_for(pkt.len()).await;
            sock_wr.send(&pkt).await?;
        }
        Ok(())
    }

    // ---------- RX: UDP -> pipeline -> TUN ----------
    pub async fn rx_loop<R, W>(&self, mut sock_rd: R, mut tun_wr: W) -> Result<()>
    where R: SockRead, W: TunWrite {
        let mut collector = crate::fec::BlockCollector::new(self.fec.data, self.fec.total());
        let mut jitter = JitterBuffer::new(self.jitter_depth, self.jitter_ms);

        loop {
            let mut datagram = sock_rd.recv().await?;
            let hdr = match OuterHeader::decode(&mut &datagram[..]) {
                Some(h) => h, None => continue,
            };

            // 1. Deobfuscate payload region
            if hdr.flags.contains(Flags::OBFUSCATED) {
                if let Some(o) = &self.obfs { o.apply(&mut datagram[OUTER_HDR_LEN..], hdr.block_id); }
            }
            let shard = datagram[OUTER_HDR_LEN..].to_vec();

            // 2. FEC: collect shards, recover when possible
            let recovered = collector.push(hdr.block_id, hdr.shard_index, shard, &self.fec)?;

            // 3. Demux + jitter reorder + write to TUN
            for inner_frame in recovered {
                let mut cur = &inner_frame[..];
                if let Some(ih) = InnerHeader::decode(&mut cur) {
                    jitter.push(ih.seq, cur.to_vec()); // stream demux keyed by ih.stream_id
                    for ready in jitter.pop_ready() {
                        tun_wr.write_frame(&ready).await?;
                    }
                }
            }
        }
    }
}

// Trait stubs — implemented in tun.rs / transport.rs
pub trait TunRead  { async fn read_frame(&mut self) -> Result<Vec<u8>>; }
pub trait TunWrite { async fn write_frame(&mut self, f: &[u8]) -> Result<()>; }
pub trait SockRead { async fn recv(&mut self) -> Result<Vec<u8>>; }
pub trait SockWrite{ async fn send(&mut self, b: &[u8]) -> Result<()>; }
