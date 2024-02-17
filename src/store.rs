use tokio::{
    sync::{mpsc, oneshot},
    net::TcpStream,
    io::AsyncWriteExt
};
use std::{
    collections::HashMap,
    time::{Instant,Duration}
};

use crate::resp::Resp;

#[derive(Debug)]
pub enum StoreCmd {
    Get(String, oneshot::Sender<Option<String>>),
    Set(String, String, Option<u64>, oneshot::Sender<()>),
    NewReplica(TcpStream)
}

pub async fn store_server(mut rx: mpsc::Receiver<StoreCmd>) {
    let mut store : HashMap<String, (String, Option<Instant>)> =
        HashMap::new();
    let mut replica : Vec<TcpStream> = vec![];
    
    while let Some(message) = rx.recv().await {
        match message {
            StoreCmd::Set(key, value, timeout, tx) => {
                let timeout2 = timeout.clone().map(| t |
						   Instant::now() +
						   Duration::from_millis(t));
                store.insert(key.clone(), (value.clone(), timeout2));
                tx.send(())
                    .expect("internal server error: onshot channel");
		
		for servant in &mut replica {
		    match timeout {
			Some(timeout) => {
			    let timeout = format!("{timeout}");
			    let resp = Resp::from(["set", &key,
						   &value, "px", &timeout]);
				servant
				.write_all(resp.as_bytes())
				.await.unwrap()
			},
			None => {
			    let resp = Resp::from(["set", &key, &value]);
			    servant
				.write_all(resp.as_bytes())
				.await.unwrap()
			}
		    }
		}
            },
            StoreCmd::Get(key, tx) => {
                let Some((value, timeout)) = store.get(&key) else {
                    tx.send(None)
                        .expect("internal server error: oneshot channel");
                    continue;
                };
                if let Some(timeout) = timeout {
                    if timeout < &Instant::now() {
                        store.remove(&key);
                        tx.send(None)
                            .expect("internal server error: onshot channel");
                        continue;
                    }
                }
                tx.send(Some(value.clone()))
                    .expect("internal server error: onshot channel");
            },
	    StoreCmd::NewReplica(socket) => {
		println!("New replica registered: {socket:?}");
		replica.push(socket)
	    }
        }
    }
}
