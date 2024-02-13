mod commands;
mod store;
mod client;

use tokio::{
    net::TcpListener,
    sync::mpsc
};

use client::Client;

#[tokio::main]
async fn main() -> anyhow::Result<()> {

    let listener = TcpListener::bind("127.0.0.1:6379").await?;

    let (tx, rx) = mpsc::channel(32);
    tokio::spawn(async move {
        store::store_server(rx).await;
    });

    println!("Server running.");
    
    loop {
        let (socket, addr) = listener.accept().await?;

        let mut client = Client { addr, socket, store_tx: tx.clone() };
        tokio::spawn(async move {
            client.handle().await;
        });
    }
    
}

