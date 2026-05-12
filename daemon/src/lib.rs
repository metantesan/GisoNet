mod config;
mod dns;
mod ipc;
mod persist;
mod pid;
mod proxy;
mod state;
mod store;

use config::Config;
use state::{DNS, PROXY, RESOLVER};
use std::fs;

pub fn run() -> color_eyre::Result<()> {
    let _ = color_eyre::install();
    let cfg = Config::load();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::filter::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{},gisonet_daemon=debug", cfg.log_level).into()),
        )
        .init();

    tracing::info!(version = env!("CARGO_PKG_VERSION"), "gisonet-daemon: starting");

    rustls::crypto::ring::default_provider().install_default().expect("ring CryptoProvider");

    pid::write_pid(&cfg.pid_path)?;

    *RESOLVER.write() = Some(cfg.upstream_dns);

    let routes = store::new_store();
    let routes_path = persist::routes_path(&cfg.data_dir);
    persist::load_routes(&routes, &routes_path).ok();

    tracing::info!(
        dns = %cfg.dns_addr,
        http = %cfg.http_addr,
        https = %cfg.https_addr,
        socket = %cfg.socket_path.display(),
        data_dir = %cfg.data_dir.display(),
        upstream_dns = %cfg.upstream_dns,
        "config"
    );

    DNS.start(routes.clone(), RESOLVER.clone(), cfg.dns_addr);
    PROXY.start(routes.clone(), cfg.http_addr, cfg.https_addr, cfg.data_dir.clone());

    let sock = cfg.socket_path.clone();
    let pid_path = cfg.pid_path.clone();

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        tokio::spawn(ipc::run(routes, cfg.socket_path, routes_path));
        tokio::signal::ctrl_c().await.ok();
    });

    tracing::info!("gisonet-daemon: shutting down...");
    let _ = fs::remove_file(&sock);
    pid::remove_pid(&pid_path);
    tracing::info!("gisonet-daemon: stopped");
    Ok(())
}
