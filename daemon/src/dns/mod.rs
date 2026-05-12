mod handler;

use crate::store::SharedRoutes;
pub use handler::CustomHandler;
use parking_lot::RwLock;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::runtime::Runtime;

pub type ForwardedResolver = Arc<RwLock<Option<SocketAddr>>>;

pub struct DnsServer {
    running: Arc<RwLock<bool>>,
}

impl DnsServer {
    pub fn new() -> Self {
        Self { running: Arc::new(RwLock::new(false)) }
    }

    pub fn is_running(&self) -> bool {
        *self.running.read()
    }

    pub fn start(&self, routes: SharedRoutes, resolver: ForwardedResolver, listen_addr: SocketAddr) {
        let running = self.running.clone();
        *running.write() = true;

        std::thread::spawn(move || {
            let rt = Runtime::new().expect("dns tokio runtime");

            let handler = CustomHandler { routes, upstream_addr: resolver };
            let mut server = hickory_server::ServerFuture::new(handler);

            rt.block_on(async {
                match tokio::net::UdpSocket::bind(listen_addr).await {
                    Ok(sock) => {
                        tracing::info!(addr = %listen_addr, "dns: server started");
                        server.register_socket(sock);
                    }
                    Err(e) => {
                        tracing::error!(%listen_addr, %e, "dns: bind failed");
                        *running.write() = false;
                        return;
                    }
                }
                tracing::debug!("dns: entering event loop");
                if let Err(e) = server.block_until_done().await {
                    tracing::error!(%e, "dns: server error");
                }
                tracing::info!("dns: server stopped");
                *running.write() = false;
            });
        });
    }
}
