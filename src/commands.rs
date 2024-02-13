    
#[derive(Debug)]
pub enum Command<'a> {
    Commands,
    Ping,
    Echo(&'a str),
    Get(&'a str),
    Set(&'a str, &'a str, Option<u64>),
    Info(Option<Section>),
    Replconf,
    Psync(Option<&'a str>, i32)
}

#[derive(Debug)]
pub enum Section {
    Replication
}

impl<'a> TryFrom<&'a str> for Command<'a> {
    type Error = &'static str;

    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        let tbl : Vec<&str> = s.split("\r\n").collect();
        let (cmd, xs) = match &tbl[..] {
            [_, _, cmd, xs@..] => (cmd, xs),
            _ => {
                return Err("command parsing failed");
            }
        };

        match (&cmd.to_lowercase()[..], xs) {
            ("command", _) => Ok(Command::Commands),
            ("ping", _) => Ok(Command::Ping),
            ("echo", [_, msg, ""]) => Ok(Command::Echo(msg)),
            ("get", [_, key, ""]) => Ok(Command::Get(key)),
            ("set", [_, key, _, value, ""]) =>
                Ok(Command::Set(key, value, None)),
            ("set", [_, key, _, value, _, "px", _, timeout, ""]) => {
                let Ok(timeout) : Result<u64,_> = timeout.parse() else {
                    return Err("timeout is not a number");
                };
                println!("set with timeout: {timeout}");                
                Ok(Command::Set(key, value, Some(timeout)))
            },
            ("info", xs) => {
                let section = match xs {
                    [""] => None,
                    [_, "replication", ""] => Some(Section::Replication),
                    [_, "REPLICATION", ""] => Some(Section::Replication),
                    _ => {
                        return Err("info: invalid section ");
                    }
                };
                Ok(Command::Info(section))
            },
	    ("replconf", _xs) => {
		println!("~=> {xs:#?}");
		Ok(Command::Replconf)
	    },
	    ("psync", ["$1", s, "$2", "-1", ""]) =>
		Ok(Command::Psync(if s == &"?" { None } else { Some(s) }, -1)),
            _ => Err("command parsing failed")
        }
    }
}
