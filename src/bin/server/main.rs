mod network;

use log::info;
use log::{LevelFilter};
use std::env;
use network::process;
use tokio::net::TcpListener;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    logger::init(LevelFilter::max()).unwrap();

    let server_addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());

    let mut listener = TcpListener::bind(&server_addr).await?;
    info!("listening on {}", &server_addr);

    loop {
        let (socket, client_addr) = listener.accept().await?;
        info!("[{:?}] connected!", client_addr);

        tokio::spawn(async move {
            process(socket).await;
        });
    }
}
