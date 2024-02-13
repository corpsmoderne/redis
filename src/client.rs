use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::{mpsc, oneshot}
};
use std::net::SocketAddr;
use crate::commands::Command;
use crate::store::StoreCmd;

pub struct Client {
    pub addr: SocketAddr,
    pub socket: TcpStream,
    pub store_tx: mpsc::Sender<StoreCmd>
}

impl Client {

    pub async fn handle(&mut self) {
        let mut buff = vec![0 ; 512];

        println!("Client {:?} connected.", self.addr);
        
        loop {
            let size = self.socket.read(&mut buff)
                .await
                .expect("fail to read data");
            if size == 0 {
                break;
            }
            let s = String::from_utf8((buff[0..size]).to_vec())
                .expect("not utf8");
            
            match Command::try_from(s.as_str()) {
                Ok(Command::Commands) => {
                    self.socket.write_all(b"+OK\r\n")
                        .await
                        .expect("fail to send data");
                }
                Ok(Command::Ping) => {
                    self.socket.write_all(b"+PONG\r\n")
                        .await
                        .expect("fail to send data");
                }
                Ok(Command::Echo(msg)) =>
                    self.send_echo(msg).await,
                Ok(Command::Get(key)) =>
                    self.handle_get(key).await,
                Ok(Command::Set(key, value, timeout)) =>
                    self.handle_set(key, value, timeout).await,
                Err(err) => {
                    println!("=> {s}");
                    self.send_error(err).await
                }                
            }
        }
        println!("Client {:?} disconnected.", self.addr);
    }

    async fn handle_get(&mut self, key: &str) {
        let (tx, rx) = oneshot::channel();
        self.store_tx.send(StoreCmd::Get(key.to_string(), tx))
            .await
            .expect("internal server error (channel)");
        let resp = if let Some(value) = rx.await.unwrap() {
            format!("${}\r\n{}\r\n", value.len(), value)
        } else {
            "$-1\r\n".to_string()
        };
        self.socket.write_all(resp.as_bytes())
            .await
            .expect("fail to send data");
    }

    async fn handle_set(
        &mut self, key: &str,
        value: &str,
        timeout: Option<u64>
    ) {
        let (tx, rx) = oneshot::channel();
        self.store_tx.send(StoreCmd::Set(key.to_string(),
                                         value.to_string(),
                                         timeout,
                                         tx))
            .await
            .expect("internal server error (channel)");
        rx.await.unwrap();
        self.socket.write_all(b"+OK\r\n")
            .await
            .expect("fail to send data");
    }
    
    async fn send_echo(&mut self, msg: &str) {
        println!("Echo: {msg}");
        let msg = format!("${}\r\n{}\r\n", msg.len(), msg);
        self.socket.write_all(msg.as_bytes())
            .await
            .expect("can't send data");
    }

    async fn send_error(&mut self, err: &str) {
        println!("Error: {err}");
        let berr = format!("-Error: {err}\r\n");
        self.socket.write_all(berr.as_bytes())
            .await
            .expect("can't send data");
    }
}
