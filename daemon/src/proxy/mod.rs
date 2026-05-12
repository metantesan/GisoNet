mod cert;
mod handler;

use crate::store::SharedRoutes;
pub use cert::{DynamicCertResolver, ensure_root_ca};
pub use handler::proxy_handler;
use parking_lot::RwLock;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::runtime::Runtime;

pub struct ProxyServer {
    running: Arc<RwLock<bool>>,
}

impl ProxyServer {
    pub fn new() -> Self {
        Self { running: Arc::new(RwLock::new(false)) }
    }

    pub fn is_running(&self) -> bool {
        *self.running.read()
    }

    pub fn start(&self, routes: SharedRoutes, http_addr: SocketAddr, https_addr: SocketAddr, data_dir: PathBuf) {
        let running = self.running.clone();
        *running.write() = true;

        std::thread::spawn(move || {
            let rt = Runtime::new().expect("proxy tokio runtime");
            rt.block_on(async move {
                let client = reqwest::Client::default();
                let (_ca_cert, ca_key) = ensure_root_ca(&data_dir);

                let resolver = Arc::new(DynamicCertResolver { ca_key, data_dir });
                let tls_config = rustls::ServerConfig::builder().with_no_client_auth().with_cert_resolver(resolver);

                let https_addr_str = https_addr.to_string();
                let http_addr_str = http_addr.to_string();

                tracing::info!(http = %http_addr_str, https = %https_addr_str, "proxy: server started");
                match actix_web::HttpServer::new(move || {
                    actix_web::App::new()
                        .app_data(actix_web::web::Data::new(client.clone()))
                        .app_data(actix_web::web::Data::new(routes.clone()))
                        .route("/{tail:.*}", actix_web::web::route().to(proxy_handler))
                })
                .bind_rustls_0_23(https_addr_str.as_str(), tls_config)
                .expect("bind https")
                .bind(http_addr_str.as_str())
                .expect("bind http")
                .run()
                .await
                {
                    Ok(_) => {}
                    Err(e) => tracing::error!(%e, "proxy: server error"),
                }
                tracing::info!("proxy: server stopped");
                *running.write() = false;
            });
        });
    }
}
