use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    // Uncomment this block to pass the first stage

    let listener = TcpListener::bind("127.0.0.1:6379").await?;

    loop {
        let (mut socket, _) = listener.accept().await?;

        tokio::spawn(async move {
            let mut buff = vec![0 ; 512];

            loop {
                let size = socket.read(&mut buff)
                    .await
                    .expect("fail to read data");
                if size == 0 {
                    break;
                }
                let s = String::from_utf8((&buff[0..size]).to_vec())
                    .expect("not utf8");
                println!("{size} -> {s:?}");
                
                let tbl : Vec<&str> = s.split("\r\n").collect();

                
                match &tbl[..] {
                    [_first, _second, "ping",_] => {
                
                        socket.write_all(b"+PONG\r\n")
                            .await
                            .expect("fail to send data");
                        
                    },
                    _ => {
                        println!("unknown command");
                    }
                }

            }
            println!("Client disconnected.");
        });
    }
    
}
