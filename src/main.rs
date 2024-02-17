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
//use std::net::SocketAddr;
use crate::store::StoreCmd;

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
	let tx2 = tx.clone();
        tokio::spawn(async move {
            servant_handshake(conf2, tx2).await;
        });
    }
    
    loop {
        let (socket, addr) = listener.accept().await?;

        let client = Client { addr, socket,
                              store_tx: tx.clone(),
                              conf: conf.clone(),
			      replica: false
        };
        tokio::spawn(async move {
            client.handle().await;
        });
    }
    
}

async fn servant_handshake(conf: Arc<Conf>, tx: mpsc::Sender<StoreCmd>) {
    let listen_port = conf.port;
    let Role::Servant { ref host, ref port } = conf.role else {
        panic!("can't be there");
    };
    let host = if host == "localhost" {
	"127.0.0.1"
    } else {
	host
    };
    
    let master_addr = format!("{host}:{port}");
    let mut socket = TcpStream::connect(&master_addr).await
        .expect("Can't establish connection with master");

    socket.write_all(Resp::from(["ping"]).as_bytes()).await
        .expect("Can't send handshake");
    
    let mut buff = vec![0 ; 512 * 10];
    let size = socket.read(&mut buff)
        .await
        .expect("Can't recieve handshake");
    
    if &buff[0..size] != Resp::pong().as_bytes() {
        panic!("bad pong handshake");
    }
    
    let listen_port = listen_port.to_string();
    socket.write_all(
        Resp::from(["REPLCONF", "listening-port", &listen_port]).as_bytes()
    )
	.await
	.expect("Can't send REPLCONF");
    let size = socket.read(&mut buff)
        .await
        .expect("Can't recieve handshake");
    if &buff[0..size] != Resp::ok().as_bytes() {
        panic!("bad REPLCONF listening-port handshake");
    }    

    socket.write_all(
        Resp::from(["REPLCONF", "capa", "psync2"]).as_bytes()
    )
	.await
	.expect("Can't send REPLCONF");
    let size = socket.read(&mut buff)
        .await
        .expect("Can't recieve handshake");
    if &buff[0..size] != Resp::ok().as_bytes() {
        panic!("bad capa handshake");
    }    

    socket.write_all(
        Resp::from(["PSYNC", "?", "-1"]).as_bytes()
    )
	.await
	.expect("Can't send REPLCONF");
    let size = socket.read(&mut buff)
        .await
        .expect("Can't recieve handshake");

    let s = String::from_utf8((buff[0..size]).to_vec())
        .expect("not utf8");
    println!("=> {s:?}");

    let a = read_all(&mut socket).await;
    let mut xs = a.split(| b | b == &b'\n');
    let Some(first) = xs.next() else {
	panic!("nope");
    };
    let Ok(s) = String::from_utf8(first.to_vec()) else {
	panic!("nope");
    };
    println!("== {s:?}");
    
    let client = Client { addr: (master_addr[..]).parse().unwrap(),
			  socket,
                          store_tx: tx.clone(),
                          conf,
			  replica: true
    };
    client.handle().await;
}

async fn read_all(socket: &mut TcpStream) -> Vec<u8> {
    let mut res = vec![];
    let mut buff = vec![0 ; 512];

    loop {
	let size = socket.read(&mut buff)
            .await
            .expect("Can't recieve handshake");
	if size < buff.len() {
	    break;
	}
	res.append(&mut buff);
	buff = vec![0 ; 512];
    }
    res.append(&mut buff);
    res
}
