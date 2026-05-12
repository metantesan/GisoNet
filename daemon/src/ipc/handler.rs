use crate::persist::save_routes;
use crate::state;
use crate::store::{SharedRoutes, add_route, remove_route};
use gisonet_common::{Request, Response, RouteEntry};
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::sync::mpsc;

pub async fn handle_client(mut stream: UnixStream, store: SharedRoutes, routes_path: PathBuf, stop_tx: mpsc::Sender<()>) {
    loop {
        let mut len_buf = [0u8; 4];
        if stream.read_exact(&mut len_buf).await.is_err() {
            return;
        }
        let msg_len = u32::from_be_bytes(len_buf) as usize;

        let mut buf = vec![0u8; msg_len];
        if stream.read_exact(&mut buf).await.is_err() {
            return;
        }

        let req: Request = match serde_json::from_slice(&buf) {
            Ok(r) => r,
            Err(_) => {
                send(&mut stream, &Response::Error { msg: "invalid request".into() }).await;
                continue;
            }
        };

        let (resp, should_stop) = dispatch(req, &store);
        send(&mut stream, &resp).await;

        if matches!(resp, Response::Ok) {
            let _ = save_routes(&store, &routes_path);
        }

        if should_stop {
            let _ = stop_tx.send(()).await;
            return;
        }
    }
}

async fn send(stream: &mut UnixStream, resp: &Response) {
    let json = serde_json::to_string(resp).unwrap();
    let len = json.len() as u32;
    let _ = stream.write_all(&len.to_be_bytes()).await;
    let _ = stream.write_all(json.as_bytes()).await;
    let _ = stream.flush().await;
}

fn dispatch(req: Request, store: &SharedRoutes) -> (Response, bool) {
    match req {
        Request::GetRoutes => {
            tracing::debug!("ipc: GetRoutes");
            let routes = store.read().clone();
            (Response::Routes { routes }, false)
        }
        Request::AddRoute { domain, ip, path, upstream } => {
            tracing::info!(%domain, %ip, %path, %upstream, "ipc: AddRoute");
            let ip = ip.parse().unwrap_or_else(|_| "127.0.0.1".parse().unwrap());
            let ok = add_route(store, RouteEntry { domain, ip, path, upstream });
            if ok { (Response::Ok, false) } else { (Response::Error { msg: "route already exists".into() }, false) }
        }
        Request::RemoveRoute { domain, path } => {
            tracing::info!(%domain, %path, "ipc: RemoveRoute");
            if remove_route(store, &domain, &path) {
                (Response::Ok, false)
            } else {
                (Response::Error { msg: "route not found".into() }, false)
            }
        }
        Request::SetResolver { addr } => {
            tracing::info!(%addr, "ipc: SetResolver");
            state::set_resolver(&addr);
            (Response::Ok, false)
        }
        Request::GetStatus => {
            tracing::debug!("ipc: GetStatus");
            (Response::Status {
                connected: true,
                dns_running: state::is_dns_running(),
                proxy_running: state::is_proxy_running(),
            }, false)
        }
        Request::Stop => {
            tracing::info!("ipc: Stop");
            (Response::Ok, true)
        }
    }
}
