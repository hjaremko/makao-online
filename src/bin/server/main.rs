mod network;
mod server;

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
    let player_amount = env::args()
        .nth(2)
        .unwrap_or_else(|| "2".to_string())
        .parse()
        .unwrap_or_else(|_| -> i32 { 2 });

    let mut listener = TcpListener::bind(&server_addr).await?;
    info!("listening on {}", &server_addr);

    let (tx, _) = broadcast::channel(16);
    let tx2 = tx.clone();

    Broadcaster::new(tx).spawn();
    handle_connections(&mut listener, tx2).await

            // match server.game_state() {
        //     ServerState::Playing => {
        // info!("server is full");
        // }
        // ServerState::Idle => {
        // let server = Arc::clone(&server);
    // server.inc();
    //     if server.current_players() == player_amount {
    //         info!("starting new game");
    //         // server.start_game();
    //     }
    // }
    // };
}

fn get_server_addr() -> String {
    env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string())
}
