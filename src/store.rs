use tokio::sync::{mpsc, oneshot};
use std::collections::HashMap;

#[derive(Debug)]
pub enum StoreCmd {
    Get(String, oneshot::Sender<Option<String>>),
    Set(String, String, oneshot::Sender<()>)
}

pub async fn store_server(mut rx: mpsc::Receiver<StoreCmd>) {
    let mut store : HashMap<String, String> = HashMap::new();

    while let Some(message) = rx.recv().await {
        match message {
            StoreCmd::Set(key, value, tx) => {
                store.insert(key, value);
                tx.send(())
                    .expect("internal server error: onshot channel");
            },
            StoreCmd::Get(key, tx) => {
                let value = store.get(&key);
                tx.send(value.cloned())
                    .expect("internal server error: onshot channel");
            }
        }
    }
}
