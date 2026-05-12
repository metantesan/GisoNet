use crate::store::SharedRoutes;
use actix_web::{HttpRequest, HttpResponse};

pub async fn proxy_handler(
    req: HttpRequest,
    body: actix_web::web::Bytes,
    routes: actix_web::web::Data<SharedRoutes>,
    client: actix_web::web::Data<reqwest::Client>,
) -> actix_web::Result<HttpResponse> {
    let method = req.method().to_string();
    let host = req.connection_info().host().trim_end_matches('.').to_string();
    let path = req.uri().path_and_query().map(|x| x.as_str()).unwrap_or("/");

    let span = tracing::debug_span!("proxy", method = %method, host = %host, path = %path);
    let _guard = span.enter();

    let upstream = {
        let r = routes.read();
        r.iter()
            .find(|entry| {
                let domain_match = entry.domain.trim_end_matches('.') == host;
                let path_match = entry.path.is_empty() || entry.path == "/" || path.starts_with(&entry.path);
                domain_match && path_match
            })
            .map(|entry| {
                let prefix =
                    if entry.path.is_empty() || entry.path == "/" { String::new() } else { entry.path.clone() };
                let rest = if prefix.is_empty() {
                    path.to_string()
                } else if let Some(p) = path.strip_prefix(&prefix) {
                    if p.is_empty() { "/".to_string() } else { p.to_string() }
                } else {
                    path.to_string()
                };
                let base = entry.upstream.trim_end_matches('/');
                format!("{base}{rest}")
            })
    };

    match upstream {
        Some(url) => {
            tracing::info!(upstream = %url, "proxy: forwarding request");
            let mut fwd = client.request(reqwest::Method::from_bytes(req.method().as_str().as_bytes()).unwrap(), &url);
            for (h, v) in req.headers().iter() {
                fwd = fwd.header(h.as_str(), v.as_bytes());
            }
            fwd = fwd
                .header("Host", host.as_str())
                .header("X-Forwarded-Host", host.as_str())
                .header("X-Forwarded-Proto", req.connection_info().scheme())
                .header("X-Forwarded-Port", req.connection_info().host().split(':').nth(1).unwrap_or("80"));

            let resp = fwd
                .body(body)
                .send()
                .await
                .map_err(|e| actix_web::error::ErrorInternalServerError(format!("proxy: {e}")))?;

            let status = resp.status().as_u16();
            tracing::info!(status, "proxy: response received");

            let mut client_resp = HttpResponse::build(actix_web::http::StatusCode::from_u16(status).unwrap());
            for (h, v) in resp.headers().iter() {
                if let Ok(val) = v.to_str() {
                    client_resp.append_header((h.as_str(), val));
                }
            }
            let body = resp.bytes().await.unwrap_or_default().to_vec();
            Ok(client_resp.body(body))
        }
        None => {
            tracing::warn!("proxy: no matching route");
            Ok(HttpResponse::NotFound().body("no route for this domain/path"))
        }
    }
}
