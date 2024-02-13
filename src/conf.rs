use anyhow::Result;

#[derive(Debug)]
pub struct Conf {
    pub port: u16,
    pub role: Role
}

#[derive(Debug)]
pub enum Role {
    Master,
    Servant((String,u16))
}

pub fn from_args() -> Result<Conf> {
    let mut port = 6379;
    let mut role = Role::Master;
    
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
                role = Role::Servant((host, p.parse()?));
            },
            _ => anyhow::bail!("unknown option")
        }
    }
    
    Ok(Conf { port, role })
}

