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

const DB : &[u8] = include_bytes!("../db/dump.rdb");

pub struct Client {
    pub addr: SocketAddr,
    pub socket: TcpStream,
    pub store_tx: mpsc::Sender<StoreCmd>,
    pub conf: Arc<Conf>,
    pub replica: bool
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
	self.socket.write_all(DB).await.expect("can't send data");
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
	if !self.replica {
	    self.send(Resp::ok()).await
	}
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
