use tokio::{
    sync::{mpsc, oneshot,RwLock},
    net::TcpStream,
    io::AsyncWriteExt
};
use std::{
    collections::HashMap,
    time::{Instant,Duration}
};
use std::sync::{Arc};
use crate::resp::Resp;

#[derive(Debug)]
pub enum StoreCmd {
    Get(String, oneshot::Sender<Option<String>>),
    Set(String, String, Option<u64>, oneshot::Sender<()>),
    NewReplica(TcpStream)
}

pub struct Store {
    store : HashMap<String, (String, Option<Instant>)>,
    replica : Vec<Arc<RwLock<TcpStream>>>,
    rx : mpsc::Receiver<StoreCmd>
}

impl Store {
    pub async fn serve(rx: mpsc::Receiver<StoreCmd>) {
	let mut store = Self {
	    store: HashMap::new(),
	    replica: vec![],
	    rx
	};
	store.run().await
    }

    async fn run(&mut self) {
	while let Some(message) = self.rx.recv().await {
            match message {
		StoreCmd::Set(key, value, timeout, tx) =>
		    self.set(key, value, timeout, tx).await,
		StoreCmd::Get(key, tx) =>
		    self.get(key, tx).await,
		StoreCmd::NewReplica(socket) => {
		    self.replica.push(Arc::new(RwLock::new(socket)))
		}
            }
	}
    }

    async fn set(
	&mut self,
	key: String,
	value: String,
	timeout: Option<u64>,
	tx: oneshot::Sender<()>
    ) {
	let timeout2 = timeout.map(| t |
				   Instant::now() +
				   Duration::from_millis(t));
        self.store.insert(key.clone(), (value.clone(), timeout2));
        tx.send(())
	    .expect("internal server error: onshot channel");
	self.send_replicas(key, value, timeout).await;
    }

    async fn get(
	&mut self,
	key: String,
	tx: oneshot::Sender<Option<String>>
    ) {
	let Some((value, timeout)) = self.store.get(&key) else {
	    tx.send(None)
                .expect("internal server error: oneshot channel");
	    return;
        };
        let resp = match timeout {
	    Some(timeout) if timeout < &Instant::now() => None,
	    _ => Some(value.clone())
	};
	if resp.is_none() {
	    self.store.remove(&key);
	}
	tx.send(resp)
	    .expect("internal server error: onshot channel");	
    }
    
    async fn send_replicas(&mut self,
			   key: String,
			   value: String,
			   timeout: Option<u64>) {
	let resp = match timeout {
	    Some(timeout) => 
		Resp::from(["set", &key, &value, 
			    "px", timeout.to_string().as_str()]),
	    None => Resp::from(["set", &key, &value])
	};
	let resp = Arc::new(resp.as_bytes().to_vec());

	let mut set = tokio::task::JoinSet::new();	

	for servant in &self.replica {
	    let resp2 = resp.clone();
	    let servant2 = servant.clone();

	    set.spawn(async move {
		let mut s = servant2.write().await;
		s.write_all(&resp2).await.unwrap();
	    });
	}
	while let Some(res) = set.join_next().await {
	    if let Err(err) = res {
		eprintln!("Error: {err}");
	    }
	}
    }
}
