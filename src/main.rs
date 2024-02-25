mod commands;
mod store;
mod client;
mod conf;
mod resp;
mod replica;

use tokio::{
    net::TcpListener,
    sync::mpsc
};
use std::sync::Arc;
use client::Client;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let conf = Arc::new(conf::from_args()?);    
    let addr = format!("127.0.0.1:{}", conf.port);
    let listener = TcpListener::bind(&addr).await?;

    let (tx, rx) = mpsc::channel(32);
    tokio::spawn(async move {
        store::Store::serve(rx).await; 
    });

    println!("Server running on {addr}.");

    if !conf.is_master() {
        let conf2 = conf.clone();
	let tx2 = tx.clone();
        tokio::spawn(async move {
            replica::servant_handshake(conf2, tx2).await;
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

