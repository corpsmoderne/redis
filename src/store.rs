use tokio::sync::{mpsc, oneshot};
use std::collections::HashMap;
use std::time::{Instant,Duration};

#[derive(Debug)]
pub enum StoreCmd {
    Get(String, oneshot::Sender<Option<String>>),
    Set(String, String, Option<u64>, oneshot::Sender<()>)
}

pub async fn store_server(mut rx: mpsc::Receiver<StoreCmd>) {
    let mut store : HashMap<String, (String, Option<Instant>)> =
        HashMap::new();

    while let Some(message) = rx.recv().await {
        match message {
            StoreCmd::Set(key, value, timeout, tx) => {
                let timeout = timeout.map(| t |
                                          Instant::now() +
                                          Duration::from_millis(t));
                store.insert(key, (value, timeout));
                tx.send(())
                    .expect("internal server error: onshot channel");
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
            }
        }
    }
}
