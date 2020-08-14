use crate::{BoxResult, DataPacket, IoResult};
use log::{debug, error, info};
use std::io::{Error, ErrorKind};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{str, thread, time};
use tokio::io::AsyncWriteExt;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::io::*;
use tokio::sync::broadcast::{Receiver, Sender, TryRecvError};

pub async fn handle_connections(listener: &mut TcpListener, tx2: Sender<DataPacket>) -> BoxResult {
    loop {
        let (socket, client_addr) = listener.accept().await?;
        info!("[{}] connected!", client_addr);
        ClientConnection::new(socket, tx2.subscribe()).spawn();
    }
}

pub struct Broadcaster {
    tx: Sender<DataPacket>,
}

impl Broadcaster {
    pub fn new(tx: Sender<i32>) -> Self {
        Broadcaster { tx }
    }

    pub fn spawn(self) {
        debug!("spawning broadcast thread");
        tokio::spawn(async move {
            if let Err(e) = self.broadcast() {
                error!("broadcast thread failed with error: {}", e);
            }
            debug!("broadcast thread ended");
        });
    }

    fn broadcast(self) -> IoResult {
        let mut i = 0;
        loop {
            if self.send_to_channel(i).is_ok() {
                i += 1;
            }
            thread::sleep(time::Duration::from_millis(1000));
        }
    }

    fn send_to_channel(&self, data: DataPacket) -> IoResult {
        if let Err(e) = self.tx.send(data) {
            return Err(Error::new(ErrorKind::Other, format!("{:?}", e)));
        }
        Ok(())
    }
}

pub struct ClientConnection {
    stream: TcpStream,
    address: SocketAddr,
    rx: Receiver<DataPacket>,
    connected: Arc<AtomicBool>,
}

impl ClientConnection {
    pub fn new(stream: TcpStream, rx: Receiver<DataPacket>) -> Self {
        let addr = stream.peer_addr().unwrap();
        ClientConnection {
            stream,
            address: addr,
            rx,
            connected: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn spawn(self) {
        tokio::spawn(async move {
            if let Err(e) = self.handle_client() {
                error!("client thread failed with error: {}", e);
            }
        });
    }

    fn handle_client(self) -> IoResult {
        debug!("processing {}", self.address);

        let (read_str, write_str) = self.stream.into_split();
        ClientSender::new(write_str, self.rx, self.connected.clone()).spawn();
        ClientReceiver::new(read_str, self.connected).spawn();

        Ok(())
    }
}

struct ClientSender {
    write_stream: OwnedWriteHalf,
    rx: Receiver<DataPacket>,
    not_closed: Arc<AtomicBool>,
}

impl ClientSender {
    pub fn new(
        write_stream: OwnedWriteHalf,
        rx: Receiver<DataPacket>,
        not_closed: Arc<AtomicBool>,
    ) -> Self {
        ClientSender {
            write_stream,
            rx,
            not_closed,
        }
    }

    pub fn spawn(self) {
        debug!("spawning sender thread");
        tokio::spawn(async move {
            if let Err(e) = self.forward_broadcasts().await {
                error!("sender thread failed with error: {}", e);
            }
        });
    }

    async fn forward_broadcasts(mut self) -> IoResult {
        let res: IoResult = loop {
            if !self.not_closed.load(Ordering::Relaxed) {
                break Ok(());
            }

            self.try_forward_broadcast().await?;
        };
        res
    }

    async fn try_forward_broadcast(&mut self) -> IoResult {
        match self.rx.try_recv() {
            Ok(message) => self.send(message).await,
            Err(e) => match e {
                TryRecvError::Empty => Ok(()),
                TryRecvError::Closed => Err(Error::new(ErrorKind::NotConnected, "channel closed")),
                TryRecvError::Lagged(_) => {
                    debug!("broadcast lagged");
                    self.rx.try_recv().unwrap();
                    Ok(())
                }
            },
        }
    }

    async fn send(&mut self, message: DataPacket) -> IoResult {
        debug!("sending to client: {}", message);
        self.write_stream
            .write_all(format!("{}\n", message).as_ref())
            .await
    }
}

struct ClientReceiver {
    read_stream: OwnedReadHalf,
    not_closed: Arc<AtomicBool>,
}

impl ClientReceiver {
    pub fn new(read_stream: OwnedReadHalf, not_closed: Arc<AtomicBool>) -> Self {
        ClientReceiver {
            read_stream,
            not_closed,
        }
    }

    pub fn spawn(mut self) {
        debug!("spawning receiver thread");
        tokio::spawn(async move {
            if let Err(e) = self.receive().await {
                error!("receiver thread failed with error: {}", e);
                debug!("setting connection indicator to false");
                self.not_closed.swap(false, Ordering::Relaxed);
            }
        });
    }

    async fn receive(&mut self) -> IoResult {
        // todo: handle in logger instance
        // let client_addr = self.read_stream;
        let mut buffer = [0; 1024];

        loop {
            // todo: close gracefully and return Ok(())
            debug!("waiting for data");
            let n = self.read_stream.read(&mut buffer).await?;

            if n == 0 {
                break Err(Error::new(ErrorKind::InvalidData, "read 0 bytes"));
            }

            // trace!("[{}] read {} bytes", client_addr, n);

            let mut buf: String = match str::from_utf8(&buffer[0..n]) {
                Ok(str) => str.to_string(),
                Err(e) => {
                    break Err(Error::new(ErrorKind::InvalidInput, e));
                }
            };

            buf.retain(|c| c.is_alphanumeric());
            debug!("got \"{}\"", buf);
        }
    }
}
