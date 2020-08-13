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

async fn send_to_client(write_stream: &mut OwnedWriteHalf, message: i32) -> Result<(), Error> {
    debug!("sending to client: {}", message);
    write_stream
        .write_all(format!("{}\n", message).as_ref())
        .await
}

async fn try_forward_from_channel(
    write_stream: &mut OwnedWriteHalf,
    rx: &mut Receiver<i32>,
) -> Result<(), Error> {
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

fn spawn_broadcaster(write_str: OwnedWriteHalf, rx: Receiver<i32>, rec_closed: Arc<AtomicBool>) {
    tokio::spawn(async move {
        if let Err(e) = forward_broadcast(rx, write_str, rec_closed).await {
            error!("broadcast thread failed with error: {}", e);
        }
    });
}

async fn forward_broadcast(
    mut rx: Receiver<i32>,
    mut write_stream: OwnedWriteHalf,
    not_closed: Arc<AtomicBool>,
) -> Result<(), Error> {
    let res: Result<(), Error> = loop {
        if !not_closed.load(Ordering::Relaxed) {
            break Ok(());
        }

        try_forward_from_channel(&mut write_stream, &mut rx).await?;
    };

    debug!("ending sending broadcast to client");
    res
}

async fn receive_from_client(
    mut buf: &mut [u8],
    client_addr: SocketAddr,
    read_str: &mut OwnedReadHalf,
) -> Result<(), Error> {
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

pub async fn handle_client(stream: TcpStream, rx: Receiver<i32>) -> Result<(), Error> {
    let mut buf = [0; 1024];
    let client_addr = stream.peer_addr()?;

    debug!("processing {}", client_addr);

    let (mut read_str, write_str) = stream.into_split();
    let not_closed = Arc::new(AtomicBool::new(true));
    let rec_closed = not_closed.clone();

    spawn_broadcaster(write_str, rx, rec_closed);

    let res = receive_from_client(&mut buf, client_addr, &mut read_str).await;

    not_closed.swap(false, Ordering::Relaxed);
    debug!("connection closed");
    res
}
