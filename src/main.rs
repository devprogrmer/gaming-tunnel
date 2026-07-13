mod config; mod protocol; mod tun; mod transport;
mod fec; mod mux; mod jitter; mod pacing; mod crypto; mod session; mod pipeline;

use clap::Parser;
use config::{Cli, Mode};

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();
    let args = match &cli.mode {
        Mode::Server(a) | Mode::Client(a) => a.clone(),
    };
    let is_server = matches!(cli.mode, Mode::Server(_));

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(args.workers)
        .enable_all()
        .build()?;

    rt.block_on(async move {
        let sess = session::Session::new(args, is_server).await?;
        sess.run().await
    })
}
