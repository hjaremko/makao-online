use log::{debug, error};
use std::net::SocketAddr::V4;
use std::net::SocketAddr::V6;
use std::str;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio::sync::broadcast::{Receiver, TryRecvError};

pub async fn process(stream: TcpStream, mut rx: Receiver<i32>) {
    let mut buf = [0; 1024];
    let client_addr = match stream.peer_addr() {
        Ok(V4(addr)) => addr,
        Ok(V6(_)) => return,
        Err(e) => {
            error!("error getting client address, e = {:?}", e);
            return;
        }
    };

    debug!("processing {}", client_addr);

    let (mut read_str, mut write_str) = stream.into_split();
    let not_closed = Arc::new(AtomicBool::new(true));
    let rec_closed = not_closed.clone();

    tokio::spawn(async move {
        while rec_closed.load(Ordering::Relaxed) {
            match rx.try_recv() {
                Ok(got) => {
                    debug!("client received from broadcast: {}", got);
                    debug!("sending received to client...");

                    if write_str
                        .write_all(format!("{}\n", got).as_ref())
                        .await
                        .is_err()
                    {
                        error!("error sending data");
                        break;
                    }
                }
                Err(e) => match e {
                    TryRecvError::Empty => {}
                    TryRecvError::Closed => {
                        debug!("closed");
                        break;
                    }
                    TryRecvError::Lagged(_) => {
                        debug!("lagged");
                        let got = rx.try_recv().unwrap();
                        debug!("client received from broadcast: {}", got);
                    }
                },
            }
        }

        debug!("ending listening to broadcast");
    });

    loop {
        let n = match read_str.read(&mut buf).await {
            Ok(n) if n == 0 => return,
            Ok(n) => n,
            Err(e) => {
                error!("failed to read from socket; err = {:?}", e);
                break;
            }
        };

        debug!("[{}] read {} bytes", client_addr, n);

        let mut buf: String = match str::from_utf8(&buf[0..n]) {
            Ok(str) => str.to_string(),
            Err(_) => {
                error!("invalid input");
                break;
            }
        };

        buf.retain(|c| c.is_alphanumeric());
        let buf = buf.as_str();

        debug!("got {}", buf);
    }

    not_closed.swap(false, Ordering::Relaxed);
    debug!("connection closed");
}
