use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::{mpsc, oneshot}
};
use std::sync::Arc;
use std::net::SocketAddr;
use crate::commands::{Command,CommandIter,Section};
use crate::store::StoreCmd;
use crate::conf::{Role,Conf};
use crate::resp::Resp;

/*
const DB : [u8 ; 88] = [
    82,  69,  68,  73,  83,  48,  48,  49,  49, 250,   9, 114,
  101, 100, 105, 115,  45, 118, 101, 114,   5,  55,  46,  50,
   46,  48, 250,  10, 114, 101, 100, 105, 115,  45,  98, 105,
  116, 115, 192,  64, 250,   5,  99, 116, 105, 109, 101, 194,
  109,   8, 188, 101, 250,   8, 117, 115, 101, 100,  45, 109,
  101, 109, 194, 176, 196,  16,   0, 250,   8,  97, 111, 102,
   45,  98,  97, 115, 101, 192,   0, 255, 240, 110,  59, 254,
  192, 255,  90, 162
];
*/

const DB : [ u8 ; 176 ] =
[82, 69, 68, 73, 83, 48, 48, 48, 57, 250, 9, 114, 101, 100, 105, 115, 45, 118, 101, 114, 6, 54, 46, 48, 46, 49, 54, 250, 10, 114, 101, 100, 105, 115, 45, 98, 105, 116, 115, 192, 64, 250, 5, 99, 116, 105, 109, 101, 194, 225, 123, 208, 101, 250, 8, 117, 115, 101, 100, 45, 109, 101, 109, 194, 168, 72, 29, 0, 250, 14, 114, 101, 112, 108, 45, 115, 116, 114, 101, 97, 109, 45, 100, 98, 192, 0, 250, 7, 114, 101, 112, 108, 45, 105, 100, 40, 55, 49, 51, 99, 98, 53, 56, 100, 101, 51, 56, 102, 101, 101, 99, 49, 51, 98, 50, 54, 97, 49, 100, 98, 97, 57, 98, 57, 97, 97, 98, 99, 51, 49, 50, 51, 55, 55, 98, 99, 250, 11, 114, 101, 112, 108, 45, 111, 102, 102, 115, 101, 116, 192, 0, 250, 12, 97, 111, 102, 45, 112, 114, 101, 97, 109, 98, 108, 101, 192, 0, 255, 183, 227, 140, 16, 150, 136, 49, 197];

pub struct Client {
    pub addr: SocketAddr,
    pub socket: TcpStream,
    pub store_tx: mpsc::Sender<StoreCmd>,
    pub conf: Arc<Conf>
}

impl Client {

    pub async fn handle(mut self) {
        let mut buff = vec![0 ; 512];
	let addr = self.addr;
        println!("Client {:?} connected.", addr);
        
        loop {
            let size = {
		let socket = &mut self.socket;
		socket.read(&mut buff)
                    .await
                    .expect("fail to read data")
	    };
            if size == 0 {
                break;
            }
            let s = std::str::from_utf8(&buff[0..size])
                .expect("not utf8");

	    for next_cmd in CommandIter::from(s) { 
		match next_cmd {
                    Ok(Command::Commands) => 
			self.send(Resp::ok()).await,
                    Ok(Command::Ping) =>
			self.send(Resp::pong()).await,
                    Ok(Command::Echo(msg)) =>
			self.send(Resp::from(msg)).await,
                    Ok(Command::Get(key)) =>
			self.handle_get(key).await,
                    Ok(Command::Set(key, value, timeout)) =>
			self.handle_set(key, value, timeout).await,
                    Ok(Command::Info(section)) => 
			self.handle_info(section).await,
		    Ok(Command::Replconf) =>
			self.send(Resp::ok()).await,
		    Ok(Command::Psync(_,_)) =>
			return self.handle_master().await,
		    Ok(Command::Err(error)) => {
			println!("** Error: {error:#?}");
		    },
                    Err(err) => self.send_error(err).await
		}
            }
	}
        println!("Client {:?} disconnected.", addr);
    }

    async fn handle_master(mut self) {
	let Role::Master { ref repl_id, ref repl_offset } =
	    self.conf.role else {
		self.send_error("I'm not a master.").await;
		return;
	    };
	self.send(Resp::full_resync(repl_id, *repl_offset)).await;
	let header = format!("${}\r\n", DB.len()).as_bytes().to_vec();
	self.socket.write_all(&header).await.expect("can't send data");
	self.socket.write_all(&DB).await.expect("can't send data");
	self.store_tx.send(StoreCmd::NewReplica(self.socket)).await.unwrap();
    }	
    
    async fn handle_info(&mut self, _section: Option<Section>) {
        let response = match self.conf.role {
            Role::Master { ref repl_id, ref repl_offset } => {
                format!("# Replication\r\nrole:master\r\nmaster_replid:{repl_id}\r\nmaster_repl_offset:{repl_offset}\r\n")
            },
            Role::Servant { ref host, ref port } => {
                format!("# Replication\r\nrole:slave\r\nmaster_host:{host}\r\nmaster_port:{port}\r\n")
            }
        };
        self.send(Resp::from(response.as_str())).await;
    }
    
    async fn handle_get(&mut self, key: &str) {
        let (tx, rx) = oneshot::channel();
        self.store_tx.send(StoreCmd::Get(key.to_string(), tx))
            .await
            .expect("internal server error (channel)");
        let resp = if let Some(value) = rx.await.unwrap() {
	    Resp::from(value.as_str())
        } else {
	    Resp::nil()
        };
	self.send(resp).await;
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
	self.send(Resp::ok()).await
    }
    
    async fn send_error(&mut self, err: &str) {
        println!("Error: {err}");
        let berr = format!("-Error: {err}\r\n");
        self.socket.write_all(berr.as_bytes())
            .await
            .expect("can't send data");
    }

    async fn send(&mut self, resp: Resp) {
        self.socket.write_all(resp.as_bytes())
	    .await
	    .expect("can't send data")
    }
}
