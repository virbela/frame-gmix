use clap::Parser;
use config::Config;
use std::{
    net::{SocketAddr, ToSocketAddrs},
    str::FromStr,
    time::Duration,
};
use tokio::{net::TcpStream, time::sleep};
use tracing::{error, info};

use crate::handler::handle_stream;

mod codec;
mod config;
mod handler;
mod message;
mod mixer;
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long)]
    url: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("frame_mediasoup=info,frame_mediasoup_proto=trace")
        .init();
    let args: Args = Args::parse();
    let config = Config::init();
    let addr = args
        .url
        .to_socket_addrs()
        .unwrap()
        .find(|addr| addr.is_ipv4())
        .unwrap_or_else(|| {
            let default = "0.0.0.0:1188";
            info!(
                "Can not resolve ipv4 address from given url. Fall back to default value: {}",
                default
            );
            SocketAddr::from_str(default).unwrap()
        });

    tokio::spawn(async move {
        loop {
            info!("Connecting to: {}", &addr);

            match TcpStream::connect(addr).await {
                Ok(stream) => match handle_stream(stream, config.clone()).await {
                    Ok(_) => {
                        info!("Shutting down application");
                        return;
                    }
                    Err(e) => error!("Tcp handle error: {:?}", e),
                },
                Err(e) => error!("Tcp connect error: {:?}", e),
            }

            sleep(Duration::from_secs(2)).await;
        }
    })
    .await
    .expect("netsocket error");
}
