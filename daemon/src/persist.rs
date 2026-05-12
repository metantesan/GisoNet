use crate::store::SharedRoutes;
use anyhow::Result;
use gisonet_common::RouteEntry;
use std::fs;
use std::path::{Path, PathBuf};

pub fn routes_path(data_dir: &Path) -> PathBuf {
    data_dir.join("routes.json")
}

pub fn load_routes(store: &SharedRoutes, path: &Path) -> Result<()> {
    if !path.exists() {
        tracing::info!(path = %path.display(), "persist: no routes file, starting empty");
        return Ok(());
    }
    let data = fs::read_to_string(path)?;
    let routes: Vec<RouteEntry> = serde_json::from_str(&data)?;
    tracing::info!(count = routes.len(), path = %path.display(), "persist: routes loaded");
    *store.write() = routes;
    Ok(())
}

pub fn save_routes(store: &SharedRoutes, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_string_pretty(&*store.read())?;
    fs::write(path, &data)?;
    tracing::debug!(path = %path.display(), "persist: routes saved");
    Ok(())
}
