use clap::{Parser, Subcommand, ValueEnum};
use serde::Deserialize;

#[derive(Parser, Debug)]
#[command(name = "gaming-tunnel", version, about = "Gaming-Optimized UDP Tunnel")]
pub struct Cli {
    #[command(subcommand)]
    pub mode: Mode,
}

#[derive(Subcommand, Debug)]
pub enum Mode {
    /// Run as server (listener)
    Server(RunArgs),
    /// Run as client (connector)
    Client(RunArgs),
}

#[derive(Parser, Debug, Clone)]
pub struct RunArgs {
    /// Local UDP bind address (server) or local source (client)
    #[arg(long, default_value = "0.0.0.0:0")]
    pub listen: String,

    /// Remote peer address (client -> server, or reverse target)
    #[arg(long)]
    pub remote: Option<String>,

    /// TUN interface name
    #[arg(long, default_value = "gtun0")]
    pub tun_name: String,

    /// TUN local IP / CIDR
    #[arg(long, default_value = "10.10.0.1/24")]
    pub tun_addr: String,

    /// MTU for the TUN device
    #[arg(long, default_value_t = 1400)]
    pub mtu: u16,

    /// FEC ratio as data:parity (e.g. 10:3). Set 0:0 to disable.
    #[arg(long, default_value = "10:3")]
    pub fec: String,

    /// Enable XOR obfuscation
    #[arg(long, default_value_t = true)]
    pub obfs: bool,

    /// Pre-shared key for obfuscation/keystream
    #[arg(long, default_value = "changeme")]
    pub key: String,

    /// Number of multiplexed streams
    #[arg(long, default_value_t = 1)]
    pub mux: u16,

    /// Jitter buffer depth (packets)
    #[arg(long, default_value_t = 8)]
    pub jitter: usize,

    /// Max jitter hold time in ms (playout deadline)
    #[arg(long, default_value_t = 30)]
    pub jitter_ms: u64,

    /// Pacing rate in Mbps (0 = unlimited / burst)
    #[arg(long, default_value_t = 0)]
    pub pace_mbps: u64,

    /// Worker threads for the event loop
    #[arg(long, default_value_t = 2)]
    pub workers: usize,

    /// Reverse tunneling mode (server dials out to client)
    #[arg(long, default_value_t = false)]
    pub reverse: bool,

    /// Reconnect interval on link failure (ms)
    #[arg(long, default_value_t = 2000)]
    pub reconnect_ms: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct FecRatio {
    pub data: usize,
    pub parity: usize,
}

impl FecRatio {
    pub fn parse(s: &str) -> anyhow::Result<Self> {
        let (d, p) = s.split_once(':').ok_or_else(|| anyhow::anyhow!("fec must be data:parity"))?;
        Ok(Self { data: d.parse()?, parity: p.parse()? })
    }
    pub fn enabled(&self) -> bool { self.data > 0 && self.parity > 0 }
    pub fn total(&self) -> usize { self.data + self.parity }
}
