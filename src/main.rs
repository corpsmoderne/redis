mod commands;
mod store;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpStream,TcpListener},
    sync::{mpsc, oneshot}
};
use commands::Command;
use store::{store_server,StoreCmd};

#[tokio::main]
async fn main() -> anyhow::Result<()> {

    let listener = TcpListener::bind("127.0.0.1:6379").await?;

    let (tx, rx) = mpsc::channel(32);
    tokio::spawn(async move {
        store_server(rx).await;
    });

    println!("Server running.");
    
    loop {
        let (mut socket, _) = listener.accept().await?;
        let my_tx = tx.clone();
        
        tokio::spawn(async move {
            let mut buff = vec![0 ; 512];
            println!("Client connected.");
            loop {
                let size = socket.read(&mut buff)
                    .await
                    .expect("fail to read data");
                if size == 0 {
                    break;
                }
                let s = String::from_utf8((&buff[0..size]).to_vec())
                    .expect("not utf8");

                match Command::try_from(s.as_str()) {
                    Err(err) => send_error(&mut socket, err).await,
                    Ok(Command::Ping) => {
                        socket.write_all(b"+PONG\r\n")
                            .await
                            .expect("fail to send data");
                    }
                    Ok(Command::Pong) => {
                        println!("PONG!");
                    },
                    Ok(Command::Echo(msg)) => {
                        send_echo(&mut socket, msg).await;
                    },
                    Ok(Command::Get(key)) => {
                        let (tx, rx) = oneshot::channel();
                        my_tx.send(StoreCmd::Get(key.to_string(), tx))
                            .await
                            .expect("internal server error (channel)");
                        let resp = if let Some(value) = rx.await.unwrap() {
                            format!("${}\r\n{}\r\n", value.len(), value)
                        } else {
                            "$-1\r\n".to_string()
                        };
                        socket.write_all(resp.as_bytes())
                            .await
                            .expect("fail to send data");
                    },
                    Ok(Command::Set(key, value)) => {
                        let (tx, rx) = oneshot::channel();                        
                        my_tx.send(StoreCmd::Set(key.to_string(),
                                                 value.to_string(),
                                                 tx))
                            .await
                            .expect("internal server error (channel)");
                        let _ = rx.await.unwrap();
                        socket.write_all(b"+OK\r\n")
                            .await
                            .expect("fail to send data");
                    }
                }
            }
            println!("Client disconnected.");
        });
    }
    
}

async fn send_error(socket: &mut TcpStream, err: &str) {
    println!("Error: {err}");
    let berr = format!("-Error: {err}\r\n");
    socket.write_all(berr.as_bytes())
        .await
        .expect("can't send data");
}

async fn send_echo(socket: &mut TcpStream, msg: &str) {
    println!("Echo: {msg}");
    let msg = format!("${}\r\n{}\r\n", msg.len(), msg);
    socket.write_all(&msg.as_bytes())
        .await
        .expect("can't send data");
}
