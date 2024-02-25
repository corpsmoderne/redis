use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::mpsc
};
use std::sync::Arc;
use crate::client::Client;
use crate::conf::{Conf,Role};
use crate::resp::Resp;
use crate::store::StoreCmd;

pub async fn servant_handshake(conf: Arc<Conf>, tx: mpsc::Sender<StoreCmd>) {
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
