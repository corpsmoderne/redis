#[derive(Debug)]
pub struct CommandIter<'a> {
    s: Option<&'a str>
}

impl<'a> From<&'a str> for CommandIter<'a> {
    fn from(s: &'a str) -> Self {
	CommandIter { s: Some(s) }
    }
}

impl<'a> Iterator for CommandIter<'a> {
    type Item = Result<Command<'a>, &'static str>;

    fn next(&mut self) -> Option<Self::Item> {
	let Some(s) = self.s else {
	    return None;
	};
	if s.is_empty() || s == "\r\n" {
	    self.s = None;
	    return None;
	}
	match parse_cmd(s) {
	    Ok((result, s2)) => {
		self.s = Some(s2);
		Some(Ok(result))
	    },
	    Err(err) => {
		self.s = None;
		Some(Err(err))
	    }
	}
    }
}

#[derive(Debug)]
pub enum Command<'a> {
    Commands,
    Ping,
    Echo(&'a str),
    Err(&'a str),
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

fn parse_cmd(s: &str) -> Result<(Command<'_>, &str), &'static str> {
    println!("~~> {s:#?}");
    
    if let Some(stripped) = s.strip_prefix("-ERR ") {
	let (s1,s2) = stripped.split_once("\r\n").ok_or("parse error")?;
	return Ok((Command::Err(s1), s2));
    };
    
    let (nbr, rest) = s.split_once("\r\n").ok_or("parse error")?;
    let Some('*') = nbr.chars().next() else {
	return Err("parse error");
    };
    let nbr = nbr[1..].parse::<usize>().map_err(|_| "parse error")?;    
    let tbl : Vec<&str> = rest.splitn(nbr*2+1, "\r\n").collect();
    let (cmd, xs) = match &tbl[..] {
        [_, cmd, xs@..] => (cmd, xs),
        _ => {
            return Err("command parsing failed");
        }
    };
    
    let result = match (&cmd.to_lowercase()[..], &xs[0..xs.len()-1]) {
        ("command", _) => Ok(Command::Commands),
        ("ping", _) => Ok(Command::Ping),
        ("echo", [_, msg]) => Ok(Command::Echo(msg)),
        ("get", [_, key]) => Ok(Command::Get(key)),
        ("set", [_, key, _, value]) =>
            Ok(Command::Set(key, value, None)),
        ("set", [_, key, _, value, _, "px", _, timeout]) => {
            let Ok(timeout) : Result<u64,_> = timeout.parse() else {
                return Err("timeout is not a number");
            };
            println!("set with timeout: {timeout}");                
            Ok(Command::Set(key, value, Some(timeout)))
        },
        ("info", xs) => {
            let section = match xs {
                [] => None,
                [_, "replication"] => Some(Section::Replication),
                [_, "REPLICATION"] => Some(Section::Replication),
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
	("psync", ["$1", s, "$2", "-1"]) =>
	    Ok(Command::Psync(if s == &"?" { None } else { Some(s) }, -1)),
        _ => Err("command parsing failed")
    };
    result.map(| res | (res, xs[xs.len()-1]))

}

impl<'a> TryFrom<&'a str> for Command<'a> {
    type Error = &'static str;

    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
	parse_cmd(s).map(| (cmd,_) | cmd)
    }
}
