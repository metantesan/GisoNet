use crate::dns::{DnsServer, ForwardedResolver};
use crate::proxy::ProxyServer;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::net::SocketAddr;
use std::sync::Arc;

pub static RESOLVER: Lazy<ForwardedResolver> = Lazy::new(|| Arc::new(RwLock::new(None)));

pub static DNS: Lazy<DnsServer> = Lazy::new(DnsServer::new);
pub static PROXY: Lazy<ProxyServer> = Lazy::new(ProxyServer::new);

pub fn set_resolver(addr: &str) {
    let parsed: Option<SocketAddr> = addr.parse().ok();
    *RESOLVER.write() = parsed;
    match parsed {
        Some(addr) => tracing::info!(%addr, "state: upstream DNS resolver updated"),
        None => tracing::error!(addr, "state: invalid upstream DNS address"),
    }
}

pub fn is_dns_running() -> bool {
    DNS.is_running()
}

pub fn is_proxy_running() -> bool {
    PROXY.is_running()
}
