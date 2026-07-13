use std::collections::BTreeMap;
use tokio::time::Instant;

/// Per-stream reorder buffer with a playout deadline.
pub struct JitterBuffer {
    buf: BTreeMap<u16, (Vec<u8>, Instant)>,
    next_seq: u16,
    depth: usize,
    hold_ms: u64,
}

impl JitterBuffer {
    pub fn new(depth: usize, hold_ms: u64) -> Self {
        Self { buf: BTreeMap::new(), next_seq: 0, depth, hold_ms }
    }

    pub fn push(&mut self, seq: u16, data: Vec<u8>) {
        self.buf.insert(seq, (data, Instant::now()));
    }

    /// Pop packets that are in-order OR past their playout deadline.
    pub fn pop_ready(&mut self) -> Vec<Vec<u8>> {
        let mut out = Vec::new();
        loop {
            if let Some((data, _)) = self.buf.remove(&self.next_seq) {
                out.push(data);
                self.next_seq = self.next_seq.wrapping_add(1);
                continue;
            }
            // deadline / overflow forcing
            let expired = self.buf.iter().next().map(|(&s, (_, t))| {
                (s, t.elapsed().as_millis() as u64 >= self.hold_ms)
            });
            match expired {
                Some((s, true)) | Some((s, _)) if self.buf.len() > self.depth => {
                    self.next_seq = s;
                }
                _ => break,
            }
        }
        out
    }
}
