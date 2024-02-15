pub enum Resp {
    Static(&'static [u8]),
    Array(Vec<u8>)
}

impl Resp {
    pub fn ok() -> Self {
        Resp::Static(b"+OK\r\n")
    }

    pub fn pong() -> Self {
        Resp::Static(b"+PONG\r\n")
    }

    pub fn nil() -> Self {
        Resp::Static(b"$-1\r\n")
    }
    
    pub fn full_resync(id: &str, offset: usize) -> Self {
	let s = format!("+FULLRESYNC {id} {offset}\r\n");
	Resp::Array(s.as_bytes().to_vec())
    }

    pub fn as_bytes<'a>(&'a self) -> &[u8] {
        match self {
            Resp::Static(a) => a,
            Resp::Array(v) => &v
        }
    }
}

impl From<&str> for Resp {
    fn from(s: &str) -> Resp {
	Resp::Array(format!("${}\r\n{}\r\n", s.len(), s)
                    .as_bytes().to_vec())
    }
}

impl<const N: usize> From<[&str ; N]> for Resp {
    fn from(tbl: [&str ; N]) -> Resp {
        let mut v = format!("*{}\r\n", tbl.len())
            .as_bytes().to_vec();
        for s in tbl {
            let Resp::Array(mut s) = Resp::from(s) else {
                panic!("this can't happen");
            };
            v.append(&mut s);
        }
	Resp::Array(v)
    }
}

#[cfg(test)]
mod tests {
    use super::Resp;
    
    #[test]
    fn test_ok() {
        assert_eq!(Resp::ok().as_bytes(), b"+OK\r\n");
    }
    
    #[test]
    fn test_pong() {
        assert_eq!(Resp::pong().as_bytes(), b"+PONG\r\n");
    }
    
    #[test]
    fn test_nil() {
        assert_eq!(Resp::nil().as_bytes(), b"$-1\r\n");
    }
    
    #[test]
    fn test_str() {
        assert_eq!(Resp::from("hello").as_bytes(),
                   b"$5\r\nhello\r\n");
    }
    
    #[test]
    fn test_arrays() {
        assert_eq!(Resp::from(["foo", "bar"]).as_bytes(),
                   b"*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n");
    }
}
