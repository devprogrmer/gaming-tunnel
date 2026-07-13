/// Lightweight XOR keystream obfuscation keyed by PSK + block_id.
/// Not cryptographically secure — purpose is DPI evasion / fingerprint removal.
pub struct Obfuscator { key: Vec<u8> }

impl Obfuscator {
    pub fn new(psk: &str) -> Self { Self { key: psk.as_bytes().to_vec() } }

    pub fn apply(&self, buf: &mut [u8], block_id: u16) {
        let seed = block_id as usize;
        for (i, b) in buf.iter_mut().enumerate() {
            let k = self.key[(i + seed) % self.key.len()];
            *b ^= k ^ ((i as u8).wrapping_mul(31));
        }
    }
    // XOR is symmetric: apply() again to deobfuscate.
}
