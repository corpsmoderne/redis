use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream,TcpListener};

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
            println!("Client connected.");
            loop {
                let size = socket.read(&mut buff)
                    .await
                    .expect("fail to read data");
                if size == 0 {
                    break;
                }
                let s = String::from_utf8((&buff[0..size]).to_vec())
                    .expect("not utf8");
                
                let tbl : Vec<&str> = s.split("\r\n").collect();
                //println!("{tbl:?}");
                
                match &tbl[..] {
                    [_first, _second, "ping", ""] => {
                        socket.write_all(b"+PONG\r\n")
                            .await
                            .expect("fail to send data");
                        
                    },
                    [_first, _second, "echo", _, msg, ""] => {
                        send_echo(&mut socket, msg).await
                    },
                    [_first, _second, "ECHO", _, msg, ""] => {
                        send_echo(&mut socket, msg).await
                    },
                    
                    _ => {
                        println!("Error: unknown command: {tbl:?}");
                        socket.write_all(b"-Error : unknown command\r\n")
                            .await
                            .expect("can't send data");
                    }
                    
                }

            }
            println!("Client disconnected.");
        });
    }
    
}

async fn send_echo(socket: &mut TcpStream, msg: &str) {
    println!("Echo: {msg}");
    let msg = format!("${}\r\n{}\r\n", msg.len(), msg);
    socket.write_all(&msg.as_bytes())
        .await
        .expect("can't send data");
}
