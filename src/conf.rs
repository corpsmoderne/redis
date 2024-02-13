use anyhow::Result;

const REPL_ID : &str = "8371b4fb1155b71f4a04d3e1bc3e18c4a990aeeb";

#[derive(Debug)]
pub struct Conf {
    pub port: u16,
    pub role: Role
}

#[derive(Debug)]
pub enum Role {
    Master { repl_id: String, repl_offset: usize },
    Servant { host: String, port: u16 }
}

pub fn from_args() -> Result<Conf> {
    let mut port = 6379;
    let mut role = Role::Master { repl_id: REPL_ID.to_string(),
                                  repl_offset: 0
    };
    
    let mut args = std::env::args();
    
    args.next();
    
    loop {
        let Some(opt) = args.next() else {
            break;
        };
        match &opt[..] {
            "--port" => {
                let Some(opt) = args.next() else {
                    anyhow::bail!("--port needs a port number")
                };
                
                port = opt.parse()?;
            },
            "--replicaof" => {
                let Some(host) = args.next() else {
                    anyhow::bail!("--replicaof needs a host")
                };
                let Some(p) = args.next() else {
                    anyhow::bail!("--replicaof needs a port")
                };
                role = Role::Servant { host, port: p.parse()? }
            },
            _ => anyhow::bail!("unknown option")
        }
    }
    
    Ok(Conf { port, role })
}

