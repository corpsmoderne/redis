#[derive(Debug)]
pub struct CommandIter<'a> {
    s: &'a str
}

impl<'a> From<&'a str> for CommandIter<'a> {
    fn from(s: &'a str) -> Self {
	CommandIter { s } 
    }
}

impl<'a> Iterator for CommandIter<'a> {
    type Item = Result<Command<'a>, &'static str>;

    fn next(&mut self) -> Option<Self::Item> {
	if self.s.is_empty() || self.s == "\r\n" {
	    return None;
	}
	match parse_cmd(self.s) {
	    Ok((result, s2)) => {
		self.s = s2;
		Some(Ok(result))
	    },
	    Err(err) => {
		self.s = "";
		Some(Err(err))
	    }
	}
    }
}

#[derive(Debug,PartialEq,Eq)]
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

#[derive(Debug,PartialEq,Eq)]
pub enum Section {
    Replication
}

fn parse_cmd(s: &str) -> Result<(Command<'_>, &str), &'static str> {
    let (nbr, rest) = s.split_once("\r\n").ok_or("need 2 or more lines")?;
    match nbr.chars().next().ok_or("need 1 char on first line")? {
	'*' => { },
	'-' => return Ok((Command::Err(&nbr[1..]), rest)),
	_   => return Err("unknown RESP type")
    }
    let nbr : usize = nbr[1..].parse()
	.map_err(|_| "array size is not a number")?;
    let tbl : Vec<&str> = rest.splitn(nbr*2+1, "\r\n").collect();
    let (cmd, xs) = match &tbl[..] {
        [_, cmd, xs@..] => Ok((cmd, xs)),
        _ => Err("malformed array: can't find command")
    }?;
    let (xs, rest) = (&xs[0..xs.len()-1], xs[xs.len()-1]);
    let result = match_result(cmd, xs)?;
    Ok((result, rest))
}

fn match_result<'a>(
    cmd: &'a str,
    xs: &[&'a str]
) -> Result<Command<'a>, &'static str> {
    match (&cmd.to_lowercase()[..], xs) {
        ("command", _) => Ok(Command::Commands),
        ("ping", _) => Ok(Command::Ping),
        ("echo", [_, msg]) => Ok(Command::Echo(msg)),
        ("get", [_, key]) => Ok(Command::Get(key)),
        ("set", [_, key, _, value]) => Ok(Command::Set(key, value, None)),
        ("set", [_, key, _, value, _, "px", _, timeout]) => {
	    let timeout = timeout.parse().or(Err("timeout is not a number"))?;
            Ok(Command::Set(key, value, Some(timeout)))
        },
        ("info", xs) => Ok(Command::Info(match_info_section(xs)?)),
	("replconf", _xs) => Ok(Command::Replconf),
	("psync", ["$1", "?", "$2", "-1"]) => Ok(Command::Psync(None, -1)),
        _ => Err("command parsing failed")
    }
}

fn match_info_section(xs: &[&str]) -> Result<Option<Section>, &'static str> {
    match xs {
        [] => Ok(None),
	[_, arg] if arg.to_lowercase() == "replication" =>
	    Ok(Some(Section::Replication)),
        _ => Err("info: invalid section ")
    }
}

impl<'a> TryFrom<&'a str> for Command<'a> {
    type Error = &'static str;

    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
	parse_cmd(s).map(| (cmd,_) | cmd)
    }
}

#[cfg(test)]
mod tests {
    use super::{Command,CommandIter};
    use crate::resp::Resp;

    #[test]
    fn test_command() {
	let input : String = Resp::from(["COMMAND"])
	    .try_into().unwrap();
	assert_eq!(Command::try_from(input.as_str()), Ok(Command::Commands));
    }

    #[test]
    fn test_ping() {
	let input : String = Resp::from(["PING"])
	    .try_into().unwrap();
	assert_eq!(Command::try_from(input.as_str()), Ok(Command::Ping));
    }

    #[test]
    fn test_ping_lowercase() {
	let input : String = Resp::from(["ping"])
	    .try_into().unwrap();
	assert_eq!(Command::try_from(input.as_str()), Ok(Command::Ping));
    }
    
    #[test]
    fn test_echo() {
	let input : String = Resp::from(["ECHO", "Hello World"])
	    .try_into().unwrap();
	assert_eq!(Command::try_from(input.as_str()),
		   Ok(Command::Echo("Hello World")));
    }

    #[test]
    fn test_get() {
	let input : String = Resp::from(["GET", "foo"])
	    .try_into().unwrap();
	assert_eq!(Command::try_from(input.as_str()),
		   Ok(Command::Get("foo")));
    }

   #[test]
    fn test_set() {
	let input : String = Resp::from(["SET", "foo", "42"])
	    .try_into().unwrap();
	assert_eq!(Command::try_from(input.as_str()),
		   Ok(Command::Set("foo", "42", None)));
    }    

   #[test]
    fn test_set_timeout() {
	let input : String = Resp::from(["SET", "foo", "42", "px", "123"])
	    .try_into().unwrap();
	assert_eq!(Command::try_from(input.as_str()),
		   Ok(Command::Set("foo", "42", Some(123))));
    }    

    #[test]
    fn test_iterator() {
	let input : [String ; 2] = [
	    Resp::from(["SET", "foo", "42"]).try_into().unwrap(),
	    Resp::from(["GET", "foo"]).try_into().unwrap()
	];
	let input : String = input.join(""); 

	let mut commands = CommandIter::from(input.as_str());
	assert_eq!(commands.next(), Some(Ok(Command::Set("foo", "42", None))));
	assert_eq!(commands.next(), Some(Ok(Command::Get("foo"))));
	assert_eq!(commands.next(), None)
    }    
    
}
