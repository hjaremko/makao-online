use crate::{DataPacket, IoResult};
use log::{debug, error};
use std::io::{Error, ErrorKind};
use std::net::SocketAddr;
use std::str;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio::sync::broadcast::{Receiver, TryRecvError};

pub async fn handle_client(stream: TcpStream, rx: Receiver<DataPacket>) -> IoResult {
    let mut buf = [0; 1024];
    let client_addr = stream.peer_addr()?;

    debug!("processing {}", client_addr);

    let (mut read_str, write_str) = stream.into_split();
    let not_closed = Arc::new(AtomicBool::new(true));
    let rec_closed = not_closed.clone();

    spawn_sender(write_str, rx, rec_closed);
    let res = receive_from_client(&mut buf, client_addr, &mut read_str).await;

    not_closed.swap(false, Ordering::Relaxed);
    debug!("connection closed");
    res
}

async fn receive_from_client(
    mut buf: &mut [u8],
    client_addr: SocketAddr,
    read_str: &mut OwnedReadHalf,
) -> IoResult {
    loop {
        // TODO: close gracefully and return Ok(())
        let n = read_str.read(&mut buf).await?;

        if n == 0 {
            break Err(Error::new(ErrorKind::InvalidData, "read 0 bytes"));
        }

        debug!("[{}] read {} bytes", client_addr, n);

        let mut buf: String = match str::from_utf8(&buf[0..n]) {
            Ok(str) => str.to_string(),
            Err(e) => {
                break Err(Error::new(ErrorKind::InvalidInput, e));
            }
        };

        buf.retain(|c| c.is_alphanumeric());
        debug!("got {}", buf);
    }
}

fn spawn_sender(write_str: OwnedWriteHalf, rx: Receiver<DataPacket>, rec_closed: Arc<AtomicBool>) {
    tokio::spawn(async move {
        if let Err(e) = forward_broadcasts(rx, write_str, rec_closed).await {
            error!("sender thread failed with error: {}", e);
        }
    });
}

async fn forward_broadcasts(
    mut rx: Receiver<DataPacket>,
    mut write_stream: OwnedWriteHalf,
    not_closed: Arc<AtomicBool>,
) -> IoResult {
    let res: IoResult = loop {
        if !not_closed.load(Ordering::Relaxed) {
            break Ok(());
        }

        try_forward_broadcast(&mut write_stream, &mut rx).await?;
    };

    debug!("ending sending broadcast to client");
    res
}

async fn try_forward_broadcast(
    write_stream: &mut OwnedWriteHalf,
    rx: &mut Receiver<DataPacket>,
) -> IoResult {
    match rx.try_recv() {
        Ok(message) => send_to_client(write_stream, message).await,
        Err(e) => match e {
            TryRecvError::Empty => Ok(()),
            TryRecvError::Closed => Err(Error::new(ErrorKind::NotConnected, "channel closed")),
            TryRecvError::Lagged(_) => {
                debug!("broadcast lagged");
                rx.try_recv().unwrap();
                Ok(())
            }
        },
    }
}

async fn send_to_client(write_stream: &mut OwnedWriteHalf, message: DataPacket) -> IoResult {
    debug!("sending to client: {}", message);
    write_stream
        .write_all(format!("{}\n", message).as_ref())
        .await
}
