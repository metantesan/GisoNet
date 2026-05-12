mod handler;

use crate::store::SharedRoutes;
pub use handler::handle_client;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use tokio::net::UnixListener;

pub async fn run(store: SharedRoutes, socket_path: PathBuf, routes_path: PathBuf) {
    let _ = fs::remove_file(&socket_path);

    let listener = match UnixListener::bind(&socket_path) {
        Ok(l) => {
            fs::set_permissions(&socket_path, PermissionsExt::from_mode(0o666)).ok();
            l
        }
        Err(e) => {
            let tmp = PathBuf::from("/tmp/gisonet.sock");
            fs::remove_file(&tmp).ok();
            match UnixListener::bind(&tmp) {
                Ok(l) => {
                    fs::set_permissions(&tmp, PermissionsExt::from_mode(0o666)).ok();
                    tracing::warn!(path = %socket_path.display(), %e, "ipc: fell back to /tmp socket");
                    l
                }
                Err(e2) => {
                    tracing::error!(%e, %e2, "ipc: cannot bind any socket");
                    return;
                }
            }
        }
    };

    tracing::info!(path = %socket_path.display(), "ipc: listening");

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                tracing::debug!(peer = ?addr, "ipc: client connected");
                let st = store.clone();
                let rp = routes_path.clone();
                tokio::spawn(async move { handle_client(stream, st, rp).await });
            }
            Err(e) => tracing::error!(%e, "ipc: accept failed"),
        }
    }
}
