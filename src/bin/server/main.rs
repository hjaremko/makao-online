mod network;

use crate::network::{handle_connections, Broadcaster};
use log::{info, LevelFilter};
use std::env;
use std::io::Error;
use tokio::net::TcpListener;
use tokio::sync::broadcast;

type DataPacket = i32;
type BoxError = Box<dyn std::error::Error>;
type BoxResult = Result<(), BoxError>;
type IoResult = Result<(), Error>;

#[tokio::main]
pub async fn main() -> BoxResult {
    logger::init(LevelFilter::max()).unwrap();
    info!("Makao Online Server");

    let server_addr = get_server_addr();
    let mut listener = TcpListener::bind(&server_addr).await?;
    info!("listening on {}", &server_addr);

    let (tx, _) = broadcast::channel(16);
    let tx2 = tx.clone();

    Broadcaster::new(tx).spawn();
    handle_connections(&mut listener, tx2).await
}

fn get_server_addr() -> String {
    env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string())
}
