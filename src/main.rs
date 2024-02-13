mod commands;
mod store;
mod client;
mod conf;

use tokio::{
    net::TcpListener,
    sync::mpsc
};
use std::sync::Arc;
use client::Client;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let conf = Arc::new(conf::from_args()?);

    println!("{conf:#?}");
    
    let addr = format!("127.0.0.1:{}", conf.port);
    let listener = TcpListener::bind(&addr).await?;

    let (tx, rx) = mpsc::channel(32);
    tokio::spawn(async move {
        store::store_server(rx).await;
    });

    println!("Server running on {addr}.");
    
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

