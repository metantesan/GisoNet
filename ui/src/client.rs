use anyhow::{Result, anyhow};
use gisonet_common::{Request, Response, RouteEntry};
use std::io::{BufReader, Read, Write};

// ---------------------------------------------------------------------------
// Platform transport
// ---------------------------------------------------------------------------

#[cfg(windows)]
use std::net::TcpStream as Transport;
#[cfg(unix)]
use std::os::unix::net::UnixStream as Transport;

#[cfg(unix)]
fn connect_transport() -> Result<Transport> {
    let paths = ["/var/run/gisonet.sock", "/tmp/gisonet.sock", "/var/run/com.metantesan.gisonet.sock"];
    for p in &paths {
        if let Ok(s) = Transport::connect(p) {
            return Ok(s);
        }
    }
    Err(anyhow!("no daemon socket found (tried: {})", paths.join(", ")))
}

#[cfg(windows)]
fn connect_transport() -> Result<Transport> {
    Transport::connect("127.0.0.1:59321").map_err(|e| anyhow!("{e}"))
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

pub struct DaemonClient {
    reader: BufReader<Transport>,
}

impl DaemonClient {
    pub fn connect() -> Result<Self> {
        let transport = connect_transport()?;
        let reader = BufReader::new(transport);
        Ok(Self { reader })
    }

    pub fn send(&mut self, req: &Request) -> Result<Response> {
        let json = serde_json::to_string(req)?;
        // length-prefixed: 4-byte big-endian length + JSON
        let len = json.len() as u32;
        let header = len.to_be_bytes();
        self.reader.get_mut().write_all(&header)?;
        self.reader.get_mut().write_all(json.as_bytes())?;
        self.reader.get_mut().flush()?;

        // read response: 4-byte length + JSON
        let mut len_buf = [0u8; 4];
        self.reader.read_exact(&mut len_buf)?;
        let resp_len = u32::from_be_bytes(len_buf) as usize;
        let mut resp_buf = vec![0u8; resp_len];
        self.reader.read_exact(&mut resp_buf)?;

        let resp: Response = serde_json::from_slice(&resp_buf)?;
        Ok(resp)
    }

    pub fn get_routes(&mut self) -> Result<Vec<RouteEntry>> {
        match self.send(&Request::GetRoutes)? {
            Response::Routes { routes } => Ok(routes),
            Response::Error { msg } => Err(anyhow!("daemon error: {msg}")),
            _ => Err(anyhow!("unexpected response")),
        }
    }

    pub fn add_route(&mut self, domain: &str, ip: &str, path: &str, upstream: &str) -> Result<()> {
        let resp = self.send(&Request::AddRoute {
            domain: domain.into(),
            ip: ip.into(),
            path: path.into(),
            upstream: upstream.into(),
        })?;
        match resp {
            Response::Ok => Ok(()),
            Response::Error { msg } => Err(anyhow!("{msg}")),
            _ => Err(anyhow!("unexpected response")),
        }
    }

    pub fn remove_route(&mut self, domain: &str, path: &str) -> Result<()> {
        let resp = self.send(&Request::RemoveRoute { domain: domain.into(), path: path.into() })?;
        match resp {
            Response::Ok => Ok(()),
            Response::Error { msg } => Err(anyhow!("{msg}")),
            _ => Err(anyhow!("unexpected response")),
        }
    }

    pub fn set_resolver(&mut self, addr: &str) -> Result<()> {
        let resp = self.send(&Request::SetResolver { addr: addr.into() })?;
        match resp {
            Response::Ok => Ok(()),
            Response::Error { msg } => Err(anyhow!("{msg}")),
            _ => Err(anyhow!("unexpected response")),
        }
    }

    #[allow(dead_code)]
    pub fn get_status(&mut self) -> Result<(bool, bool)> {
        match self.send(&Request::GetStatus)? {
            Response::Status { dns_running, proxy_running, .. } => Ok((dns_running, proxy_running)),
            Response::Error { msg } => Err(anyhow!("{msg}")),
            _ => Err(anyhow!("unexpected response")),
        }
    }
}
