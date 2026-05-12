use serde::{Deserialize, Serialize};
use std::net::IpAddr;

fn default_ip() -> IpAddr {
    "127.0.0.1".parse().unwrap()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteEntry {
    pub domain: String,
    #[serde(default = "default_ip")]
    pub ip: IpAddr,
    pub path: String,
    pub upstream: String,
}
