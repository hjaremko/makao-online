use tokio::prelude::*;
use tokio::net::TcpStream;
use log::{error, debug};
use std::str;
use std::net::Shutdown;


pub async fn process(mut stream: TcpStream) {
    let mut buf = [0; 1024];

    loop {
        let n = match stream.read(&mut buf).await {
            Ok(n) if n == 0 => return,
            Ok(n) => n,
            Err(e) => {
                error!("failed to read from socket; err = {:?}", e);
                return;
            }
        };

        let client_addr = match stream.local_addr() {
            Ok(addr) => addr,
            Err(e) => {
                error!("error getting client address, e = {:?}", e);
                return;
            }
        };

        debug!("[{}] read {} bytes", client_addr, n);

        if let Err(e) = stream.write_all(&buf[0..n]).await {
            error!("failed to write to socket; err = {:?}", e);
            return;
        }

        match str::from_utf8(&buf[0..n]) {
            Ok(as_utf8) => {
                debug!("[{:?}] wrote {:?}", client_addr, as_utf8);
            }
            Err(e) => {
                debug!("error parsing message, e = {:?}", e);
                stream.shutdown(Shutdown::Both).unwrap();
                return;
            }
        };
    }
}
