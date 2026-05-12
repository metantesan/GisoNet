use crate::persist::save_routes;
use crate::state;
use crate::store::{SharedRoutes, add_route, remove_route};
use gisonet_common::{Request, Response, RouteEntry};
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

pub async fn handle_client(mut stream: UnixStream, store: SharedRoutes, routes_path: PathBuf) {
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

        let resp = dispatch(req, &store);
        send(&mut stream, &resp).await;

        if matches!(resp, Response::Ok) {
            let _ = save_routes(&store, &routes_path);
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

fn dispatch(req: Request, store: &SharedRoutes) -> Response {
    match req {
        Request::GetRoutes => {
            tracing::debug!("ipc: GetRoutes");
            let routes = store.read().clone();
            Response::Routes { routes }
        }
        Request::AddRoute { domain, ip, path, upstream } => {
            tracing::info!(%domain, %ip, %path, %upstream, "ipc: AddRoute");
            let ip = ip.parse().unwrap_or_else(|_| "127.0.0.1".parse().unwrap());
            let ok = add_route(store, RouteEntry { domain, ip, path, upstream });
            if ok { Response::Ok } else { Response::Error { msg: "route already exists".into() } }
        }
        Request::RemoveRoute { domain, path } => {
            tracing::info!(%domain, %path, "ipc: RemoveRoute");
            if remove_route(store, &domain, &path) {
                Response::Ok
            } else {
                Response::Error { msg: "route not found".into() }
            }
        }
        Request::SetResolver { addr } => {
            tracing::info!(%addr, "ipc: SetResolver");
            state::set_resolver(&addr);
            Response::Ok
        }
        Request::GetStatus => {
            tracing::debug!("ipc: GetStatus");
            Response::Status {
                connected: true,
                dns_running: state::is_dns_running(),
                proxy_running: state::is_proxy_running(),
            }
        }
    }
}
