use crate::client::DaemonClient;
use crate::tray::TrayMsg;
use gisonet_common::RouteEntry;
use gpui::*;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum InputField {
    Domain,
    Ip,
    Path,
    Upstream,
    Resolver,
}

pub(crate) struct GisoNet {
    pub(crate) routes: Vec<RouteEntry>,
    pub(crate) daemon_connected: bool,
    pub(crate) domain: String,
    pub(crate) ip: String,
    pub(crate) path: String,
    pub(crate) upstream: String,
    pub(crate) resolver: String,
    pub(crate) active: Option<InputField>,
    pub(crate) daemon: Arc<Mutex<Option<DaemonClient>>>,
    pub(crate) tray_rx: mpsc::Receiver<TrayMsg>,
    pub(crate) focus_handle: FocusHandle,
    pub(crate) last_poll: Instant,
    pub(crate) status_message: String,
}

impl GisoNet {
    pub(crate) fn poll_daemon(&mut self) {
        let mut g = self.daemon.lock().unwrap();
        match g.as_mut() {
            Some(c) => {
                let c: &mut DaemonClient = c;
                match c.get_routes() {
                    Ok(routes) => {
                        self.daemon_connected = true;
                        self.routes = routes;
                    }
                    Err(_) => {
                        self.daemon_connected = false;
                        *g = None;
                    }
                }
            }
            None => {
                *g = DaemonClient::connect().ok();
                self.daemon_connected = g.is_some();
            }
        }
    }

    pub(crate) fn handle_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let keystroke = &event.keystroke;
        let buf = match self.active {
            Some(InputField::Domain) => &mut self.domain,
            Some(InputField::Ip) => &mut self.ip,
            Some(InputField::Path) => &mut self.path,
            Some(InputField::Upstream) => &mut self.upstream,
            Some(InputField::Resolver) => &mut self.resolver,
            None => return,
        };

        match keystroke.key.as_str() {
            "backspace" => {
                buf.pop();
                cx.notify();
            }
            "enter" => {
                if self.active == Some(InputField::Resolver) {
                    self.set_resolver();
                } else {
                    self.add_route();
                }
                cx.notify();
            }
            key => {
                if key.len() == 1 {
                    buf.push_str(key);
                    cx.notify();
                }
            }
        }
    }

    pub(crate) fn add_route(&mut self) {
        if self.domain.is_empty() {
            return;
        }
        let ip = if self.ip.is_empty() { "127.0.0.1".into() } else { self.ip.clone() };
        let mut g = self.daemon.lock().unwrap();
        if let Some(c) = g.as_mut() {
            let c: &mut DaemonClient = c;
            let _ = c.add_route(&self.domain, &ip, &self.path, &self.upstream);
            drop(g);
        } else {
            drop(g);
        }
        self.poll_daemon();
    }

    pub(crate) fn remove_route(&mut self) {
        if self.domain.is_empty() {
            return;
        }
        let mut g = self.daemon.lock().unwrap();
        if let Some(c) = g.as_mut() {
            let c: &mut DaemonClient = c;
            let _ = c.remove_route(&self.domain, &self.path);
            drop(g);
        } else {
            drop(g);
        }
        self.poll_daemon();
    }

    pub(crate) fn set_resolver(&mut self) {
        if self.resolver.is_empty() {
            return;
        }
        let mut g = self.daemon.lock().unwrap();
        if let Some(c) = g.as_mut() {
            let c: &mut DaemonClient = c;
            let _ = c.set_resolver(&self.resolver);
        }
    }

    pub(crate) fn run_daemon_with_sudo(&mut self) {
        if self.daemon_connected {
            self.status_message = "Daemon is already running.".into();
            return;
        }

        let exe = std::env::current_exe().ok();
        let daemon = exe.as_deref().unwrap_or_else(|| std::path::Path::new("gisonet"));

        #[cfg(target_os = "linux")]
        {
            match std::process::Command::new("pkexec").arg(daemon).arg("--daemon").spawn() {
                Ok(_) => {
                    self.status_message = "Daemon launching via pkexec...".into();
                    std::thread::sleep(Duration::from_millis(500));
                    self.poll_daemon();
                }
                Err(e) => {
                    self.status_message = format!(
                        "Could not launch daemon: {e}. Try 'sudo systemctl enable --now gisonet'."
                    );
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            match std::process::Command::new("sudo").arg("-b").arg(daemon).arg("--daemon").spawn() {
                Ok(_) => {
                    self.status_message = "Daemon launching via sudo...".into();
                    std::thread::sleep(Duration::from_millis(500));
                    self.poll_daemon();
                }
                Err(e) => {
                    self.status_message = format!("Could not launch daemon: {e}");
                }
            }
        }

        #[cfg(windows)]
        {
            let _ = daemon;
            self.status_message = "Run 'gisonet --daemon' from an elevated terminal.".into();
        }
    }

    pub(crate) fn check_tray(&mut self, cx: &mut Context<Self>) {
        while let Ok(msg) = self.tray_rx.try_recv() {
            match msg {
                TrayMsg::Quit => cx.quit(),
                TrayMsg::Show => {}
            }
        }

        if self.last_poll.elapsed() >= Duration::from_secs(3) {
            self.poll_daemon();
            self.last_poll = Instant::now();
        }
    }
}

impl Focusable for GisoNet {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
