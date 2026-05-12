use clap::Parser;
use serde::Deserialize;
use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Debug, Clone, Parser)]
#[command(name = "gisonet-daemon", about = "Local HTTPS dev proxy daemon")]
struct Cli {
    #[arg(short = 'c', long = "config", help = "Path to config file")]
    config: Option<PathBuf>,

    #[arg(long = "dns-port", help = "DNS listen port [default: 53]")]
    dns_port: Option<u16>,

    #[arg(long = "http-port", help = "HTTP listen port [default: 80]")]
    http_port: Option<u16>,

    #[arg(long = "https-port", help = "HTTPS listen port [default: 443]")]
    https_port: Option<u16>,

    #[arg(long = "socket", help = "Unix socket path [default: /var/run/gisonet.sock]")]
    socket: Option<PathBuf>,

    #[arg(long = "data-dir", help = "Data directory [default: /var/lib/gisonet]")]
    data_dir: Option<PathBuf>,

    #[arg(long = "upstream-dns", help = "Upstream DNS [default: 1.1.1.1:53]")]
    upstream_dns: Option<SocketAddr>,

    #[arg(long = "log-level", help = "Log level [default: info]")]
    log_level: Option<String>,

    /// Hidden flag consumed by the UI binary before reaching the daemon.
    #[arg(long = "daemon", hide = true)]
    daemon: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct ConfigFile {
    dns_port: Option<u16>,
    http_port: Option<u16>,
    https_port: Option<u16>,
    socket: Option<PathBuf>,
    data_dir: Option<PathBuf>,
    upstream_dns: Option<SocketAddr>,
    log_level: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub dns_addr: SocketAddr,
    pub http_addr: SocketAddr,
    pub https_addr: SocketAddr,
    pub socket_path: PathBuf,
    pub data_dir: PathBuf,
    pub upstream_dns: SocketAddr,
    pub log_level: String,
    pub pid_path: PathBuf,
}

impl Config {
    pub fn load() -> Self {
        let cli = Cli::parse();

        let config_file: ConfigFile = cli
            .config
            .as_ref()
            .and_then(|p| fs::read_to_string(p).ok())
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default();

        let data_dir = cli.data_dir.or(config_file.data_dir).unwrap_or_else(|| PathBuf::from("/var/lib/gisonet"));

        let socket_path = cli.socket.or(config_file.socket).unwrap_or_else(|| PathBuf::from("/var/run/gisonet.sock"));

        let upstream_dns =
            cli.upstream_dns.or(config_file.upstream_dns).unwrap_or_else(|| "1.1.1.1:53".parse().unwrap());

        let dns_port = cli.dns_port.or(config_file.dns_port).unwrap_or(53);
        let http_port = cli.http_port.or(config_file.http_port).unwrap_or(80);
        let https_port = cli.https_port.or(config_file.https_port).unwrap_or(443);

        let log_level = cli.log_level.or(config_file.log_level).unwrap_or_else(|| "info".into());

        let pid_path = data_dir.join("gisonet-daemon.pid");

        #[cfg(target_os = "linux")]
        let dns_listen = format!("127.0.0.10:{dns_port}");
        #[cfg(not(target_os = "linux"))]
        let dns_listen = format!("0.0.0.0:{dns_port}");

        Self {
            dns_addr: dns_listen.parse().unwrap(),
            http_addr: format!("0.0.0.0:{http_port}").parse().unwrap(),
            https_addr: format!("0.0.0.0:{https_port}").parse().unwrap(),
            socket_path,
            data_dir: data_dir.clone(),
            upstream_dns,
            log_level,
            pid_path,
        }
    }
}
