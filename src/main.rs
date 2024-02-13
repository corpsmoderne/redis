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
    let args : Vec<String> = std::env::args().collect();
    let args2 : Vec<&str> = args.iter()
        .map(| s | &**s)
        .collect();
    
    let port = match &args2[..] {
        [_, "--port", port] => {
            let port = port.parse::<u16>()?;
            port
        },
        _ => 6379
    };
    let addr = format!("127.0.0.1:{port}");
    let listener = TcpListener::bind(&addr).await?;

    let (tx, rx) = mpsc::channel(32);
    tokio::spawn(async move {
        store::store_server(rx).await;
    });

    println!("Server running on {addr}.");
    
    loop {
        let (socket, addr) = listener.accept().await?;

        let mut client = Client { addr, socket, store_tx: tx.clone() };
        tokio::spawn(async move {
            client.handle().await;
        });
    }
    
}

