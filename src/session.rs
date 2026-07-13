use crate::config::{RunArgs, FecRatio};
use crate::pipeline::Pipeline;
use anyhow::Result;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

pub struct Session {
    args: RunArgs,
    is_server: bool,
    fec: FecRatio,
}

impl Session {
    pub async fn new(args: RunArgs, is_server: bool) -> Result<Self> {
        let fec = FecRatio::parse(&args.fec)?;
        Ok(Self { args, is_server, fec })
    }

    pub async fn run(self) -> Result<()> {
        loop {
            match self.run_once().await {
                Ok(_) => {
                    tracing::warn!("tunnel closed cleanly, reconnecting");
                }
                Err(e) => {
                    tracing::error!("tunnel error: {e:#}, reconnecting");
                }
            }
            // Reconnect logic for lossy links
            sleep(Duration::from_millis(self.args.reconnect_ms)).await;
        }
    }

    async fn run_once(&self) -> Result<()> {
        // 1. Open TUN
        let tun_dev = crate::tun::TunDevice::create(&self.args)?;
        // 2. Open UDP transport (bind/connect depends on role + reverse)
        let sock = crate::transport::UdpTransport::establish(&self.args, self.is_server).await?;

        // 3. Build pipeline (wires FEC, mux, jitter, pacing, crypto)
        let pipeline = Arc::new(Pipeline::new(&self.args, self.fec)?);

        // 4. Spawn TX (TUN -> UDP) and RX (UDP -> TUN) tasks
        let (tun_rd, tun_wr) = tun_dev.split();
        let (sock_rd, sock_wr) = sock.split();

        let tx = {
            let p = pipeline.clone();
            tokio::spawn(async move { p.tx_loop(tun_rd, sock_wr).await })
        };
        let rx = {
            let p = pipeline.clone();
            tokio::spawn(async move { p.rx_loop(sock_rd, tun_wr).await })
        };

        // Whichever ends first triggers reconnect
        tokio::select! {
            r = tx => r??,
            r = rx => r??,
        }
        Ok(())
    }
}
