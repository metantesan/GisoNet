use color_eyre::eyre::{Result, bail};
use std::fs;
use std::path::PathBuf;

pub fn write_pid(path: &PathBuf) -> Result<()> {
    if path.exists() {
        let old = fs::read_to_string(path)?;
        let old_pid: u32 = old.trim().parse().unwrap_or(0);
        if old_pid != 0 {
            let running = fs::exists(format!("/proc/{old_pid}")).unwrap_or(false);
            if running {
                bail!("daemon already running (pid {old_pid})");
            }
            tracing::warn!(old_pid, "stale pid file, removing");
            fs::remove_file(path)?;
        }
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let pid = std::process::id();
    fs::write(path, pid.to_string())?;
    tracing::info!(pid, path = %path.display(), "wrote pid file");
    Ok(())
}

pub fn remove_pid(path: &PathBuf) {
    let _ = fs::remove_file(path);
}
