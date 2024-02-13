    
#[derive(Debug)]
pub enum Command<'a> {
    Ping,
    Pong,
    Echo(&'a str),
    Get(&'a str),
    Set(&'a str, &'a str)
}

impl<'a> TryFrom<&'a str> for Command<'a> {
    type Error = &'static str;

    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        let tbl : Vec<&str> = s.split("\r\n").collect();
        match &tbl[..] {
            ["*1", "$4", "PING", ""] => Ok(Command::Ping),
            ["*1", "$4", "ping", ""] => Ok(Command::Ping),

            ["+PONG", ""] => Ok(Command::Pong),
            ["+pong", ""] => Ok(Command::Pong),
            
            ["*2", "$4", "ECHO", _, msg, ""] => Ok(Command::Echo(msg)),
            ["*2", "$4", "echo", _, msg, ""] => Ok(Command::Echo(msg)),

            ["*2", "$3", "get", _, key, ""] => Ok(Command::Get(key)),
            ["*2", "$3", "GET", _, key, ""] => Ok(Command::Get(key)),

            ["*3", "$3", "set", _, key, _, value, ""] =>
                Ok(Command::Set(key, value)),
            ["*3", "$3", "SET", _, key, _, value, ""] =>
                Ok(Command::Set(key, value)),
            _ => Err("unknown command")
        }
        
    }
}
