use crate::route::RouteEntry;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Request {
    AddRoute { domain: String, ip: String, path: String, upstream: String },
    RemoveRoute { domain: String, path: String },
    GetRoutes,
    SetResolver { addr: String },
    GetStatus,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Response {
    Routes { routes: Vec<RouteEntry> },
    Status { connected: bool, dns_running: bool, proxy_running: bool },
    Resolver { addr: String },
    Error { msg: String },
    Ok,
}
