mod client;
mod render;
mod state;
mod tray;

use client::DaemonClient;
use gpui::*;
use state::GisoNet;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tray::tray_thread;

fn main() {
    if std::env::args().any(|a| a == "--daemon") {
        gisonet_daemon::run().unwrap();
        return;
    }

    let (tray_tx, tray_rx) = mpsc::channel();
    let daemon: Arc<Mutex<Option<DaemonClient>>> = Arc::new(Mutex::new(DaemonClient::connect().ok()));

    std::thread::Builder::new().name("tray".into()).spawn(|| tray_thread(tray_tx)).unwrap();

    Application::new().run(move |cx: &mut App| {
        let _ = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::centered(size(px(520.0), px(640.0)), cx)),
                ..Default::default()
            },
            move |_, cx| {
                let view = cx.new(|cx| {
                    let mut app = GisoNet {
                        routes: Vec::new(),
                        daemon_connected: daemon.lock().unwrap().is_some(),
                        domain: String::new(),
                        ip: String::new(),
                        path: String::new(),
                        upstream: String::new(),
                        resolver: String::new(),
                        active: None,
                        daemon: daemon.clone(),
                        tray_rx,
                        focus_handle: cx.focus_handle(),
                        last_poll: Instant::now(),
                        status_message: String::new(),
                    };
                    app.poll_daemon();
                    app
                });

                view
            },
        );
    });
}
