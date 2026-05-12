use gisonet_common::RouteEntry;
use parking_lot::RwLock;
use std::sync::Arc;

pub type SharedRoutes = Arc<RwLock<Vec<RouteEntry>>>;

pub fn new_store() -> SharedRoutes {
    Arc::new(RwLock::new(Vec::new()))
}

pub fn add_route(store: &SharedRoutes, entry: RouteEntry) -> bool {
    let mut w = store.write();
    if w.iter().any(|r| r.domain == entry.domain && r.path == entry.path) {
        tracing::warn!(domain = %entry.domain, path = %entry.path, "store: duplicate route rejected");
        return false;
    }
    w.push(entry.clone());
    tracing::info!(domain = %entry.domain, path = %entry.path, upstream = %entry.upstream, "store: route added");
    true
}

pub fn remove_route(store: &SharedRoutes, domain: &str, path: &str) -> bool {
    let mut w = store.write();
    let len = w.len();
    w.retain(|r| !(r.domain == domain && r.path == path));
    let removed = w.len() < len;
    if removed {
        tracing::info!(%domain, %path, "store: route removed");
    } else {
        tracing::warn!(%domain, %path, "store: route not found");
    }
    removed
}
