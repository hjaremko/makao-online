mod network;

use log::{debug, error, info, trace, LevelFilter};
use network::handle_client;
use std::net::SocketAddr::V4;
use std::{env, thread, time};
use tokio::net::TcpListener;
use tokio::sync::broadcast;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    logger::init(LevelFilter::max()).unwrap();

    let server_addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());

    let mut listener = TcpListener::bind(&server_addr).await?;
    info!("listening on {}", &server_addr);

    let (tx, _) = broadcast::channel(16);
    let tx2 = tx.clone();

    tokio::spawn(async move {
        debug!("broadcasting...");
        let mut i = 0;
        loop {
            match tx.send(i) {
                Ok(_) => {
                    trace!("broadcast: {}", i);
                    i += 1;
                }
                Err(e) => {
                    trace!("error broadcasting: {:?}", e);
                }
            }
            thread::sleep(time::Duration::from_millis(1000));
        }
    });

    loop {
        let (socket, client_add) = listener.accept().await?;
        let client_addr = match client_add {
            V4(a) => a,
            _ => panic!(),
        };
        info!("[{:?}] connected!", client_addr);

        let tx2 = tx2.clone();

        tokio::spawn(async move {
            let rx = tx2.subscribe();
            if let Err(e) = handle_client(socket, rx).await {
                error!("client thread failed with error: {}", e);
            }
        });
    }
}
