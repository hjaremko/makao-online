mod network;

use log::{debug, error, info, LevelFilter};
use network::handle_client;
use std::io::Error;
use std::{env, thread, time};
use tokio::io::ErrorKind;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio::sync::broadcast::Sender;

type DataPacket = i32;
type BoxError = Box<dyn std::error::Error>;
type BoxResult = Result<(), BoxError>;
type IoResult = Result<(), Error>;

#[tokio::main]
pub async fn main() -> BoxResult {
    logger::init(LevelFilter::max()).unwrap();

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

async fn handle_connections(listener: &mut TcpListener, tx2: Sender<i32>) -> BoxResult {
    loop {
        let (socket, client_addr) = listener.accept().await?;
        info!("[{:?}] connected!", client_addr);

        spawn_client_handler(socket, tx2.clone());
    }
}

fn spawn_client_handler(socket: TcpStream, tx: Sender<DataPacket>) {
    tokio::spawn(async move {
        let rx = tx.subscribe();
        if let Err(e) = handle_client(socket, rx).await {
            error!("client thread failed with error: {}", e);
        }
    });
}

struct Broadcaster {
    tx: Sender<DataPacket>,
}

impl Broadcaster {
    pub fn new(tx: Sender<i32>) -> Self {
        Broadcaster { tx }
    }

    pub fn spawn(self) {
        debug!("spawning broadcast thread");
        tokio::spawn(async move {
            if let Err(e) = self.broadcast().await {
                error!("broadcast thread failed with error: {}", e);
            }
        });
    }

    async fn broadcast(self) -> IoResult {
        let mut i = 0;
        loop {
            if self.send_to_channel(i).is_ok() {
                i += 1;
            }
            thread::sleep(time::Duration::from_millis(1000));
        }
    }

    fn send_to_channel(&self, i: DataPacket) -> IoResult {
        if let Err(e) = self.tx.send(i) {
            return Err(Error::new(ErrorKind::Other, format!("{:?}", e)));
        }
        Ok(())
    }
}
