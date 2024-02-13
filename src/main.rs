mod commands;
mod store;
mod client;
mod conf;
mod resp;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpStream,TcpListener},
    sync::mpsc
};
use std::sync::Arc;
use client::Client;
use conf::{Conf,Role};
use resp::Resp;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let conf = Arc::new(conf::from_args()?);    
    let addr = format!("127.0.0.1:{}", conf.port);
    let listener = TcpListener::bind(&addr).await?;

    let (tx, rx) = mpsc::channel(32);
    tokio::spawn(async move {
        store::store_server(rx).await;
    });

    println!("Server running on {addr}.");

    if !conf.is_master() {
        let conf2 = conf.clone();
        tokio::spawn(async move {
            servant_handshake(conf2).await;
        });
    }
    
    loop {
        let (socket, addr) = listener.accept().await?;

        let mut client = Client { addr, socket,
                                  store_tx: tx.clone(),
                                  conf: conf.clone()
        };
        tokio::spawn(async move {
            client.handle().await;
        });
    }
    
}

async fn servant_handshake(conf: Arc<Conf>) {
    let listen_port = conf.port;
    
    let Role::Servant { ref host, ref port } = conf.role else {
        panic!("can't be there");
    };

    let master_addr = format!("{host}:{port}");
    let mut socket = TcpStream::connect(master_addr).await
        .expect("Can't establish connection with master");

    socket.write_all(&Resp::from(["ping"]).to_vec()).await
        .expect("Can't send handshake");
    
    let mut buff = vec![0 ; 512];
    let size = socket.read(&mut buff)
        .await
        .expect("Can't recieve handshake");
    
    if buff[0..size] != Resp::Pong.to_vec() {
        panic!("bad pong handshake");
    }
    
    let listen_port = listen_port.to_string();
    Resp::from(["REPLCONF", "listening-port", &listen_port])
	.send_to(&mut socket)
	.await
	.expect("Can't send REPLCONF");
    let size = socket.read(&mut buff)
        .await
        .expect("Can't recieve handshake");
    if buff[0..size] != Resp::Ok.to_vec() {
        panic!("bad REPLCONF listening-port handshake");
    }    

    Resp::from(["REPLCONF", "capa", "psync2"])
	.send_to(&mut socket)
	.await
	.expect("Can't send REPLCONF");
    let size = socket.read(&mut buff)
        .await
        .expect("Can't recieve handshake");
    if buff[0..size] != Resp::Ok.to_vec() {
        panic!("bad capa handshake");
    }    

}
