use tokio::net::TcpStream;
use tokio::io::AsyncWriteExt;

pub enum Resp {
    Ok,
    Pong,
    Nil,
    Array(Vec<u8>)
}

impl Resp {
    pub fn to_vec(self) -> Vec<u8> {
	match self {
	    Resp::Ok => (b"+OK\r\n").to_vec(),
	    Resp::Pong => (b"+PONG\r\n").to_vec(),
	    Resp::Nil => (b"$-1\r\n").to_vec(),
	    Resp::Array(v) => v
	}
    }

    pub async fn send_to(&self, socket: &mut TcpStream) -> anyhow::Result<()> {
	match self {
	    Resp::Ok => socket.write_all(b"+OK\r\n").await?,
	    Resp::Pong => socket.write_all(b"+PONG\r\n").await?,
	    Resp::Nil => socket.write_all(b"$-1\r\n").await?,
	    Resp::Array(v) => socket.write_all(v).await?,
	}
	Ok(())
    }
}

impl From<&str> for Resp {
    fn from(s: &str) -> Resp {
	Resp::Array(format!("${}\r\n{}\r\n", s.len(), s).as_bytes().to_vec())
    }
}
    
impl From<[&str ; 1]> for Resp {
    fn from(tbl: [&str ; 1]) -> Resp {
	let s = cont_iter(format!("*{}\r\n", tbl.len()), tbl.into_iter());
	Resp::Array(s.as_bytes().to_vec())
    }
}

impl From<[&str ; 2]> for Resp {
    fn from(tbl: [&str ; 2]) -> Resp {
	let s = cont_iter(format!("*{}\r\n", tbl.len()), tbl.into_iter());
	Resp::Array(s.as_bytes().to_vec())
    }
}

impl From<[&str ; 3]> for Resp {
    fn from(tbl: [&str ; 3]) -> Resp {
	let s = cont_iter(format!("*{}\r\n", tbl.len()), tbl.into_iter());
	Resp::Array(s.as_bytes().to_vec())
    }
}

fn cont_iter<'a>(
    mut base: String,
    iter: impl Iterator<Item=&'a str>
) -> String {
    for s1 in iter {
	let s2 = format!("${}\r\n{}\r\n", s1.len(), s1);
	base.push_str(&s2);
    }
    base
}
