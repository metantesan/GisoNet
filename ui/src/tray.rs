use std::sync::mpsc;
use tray_menu::{Icon, MouseButton, MouseButtonState, PopupMenu, TextEntry, TrayIconBuilder, TrayIconEvent};

const APP_ICON: &[u8] = include_bytes!("../../assets/icon.png");

pub(crate) enum TrayMsg {
    Show,
    Quit,
}

pub(crate) fn tray_thread(tx: mpsc::Sender<TrayMsg>) {
    let icon = load_app_icon();

    let _tray = Box::leak(Box::new(TrayIconBuilder::new().with_icon(icon).with_tooltip("GisoNet").build().unwrap()));

    let rx = TrayIconEvent::receiver();
    loop {
        match rx.recv() {
            Ok(TrayIconEvent::Click { button: MouseButton::Left, .. }) => {
                let _ = tx.send(TrayMsg::Show);
            }
            Ok(TrayIconEvent::Click {
                button: MouseButton::Right,
                button_state: MouseButtonState::Down,
                position,
                ..
            }) => {
                let mut menu = PopupMenu::new();
                menu.add(&TextEntry::of("show", "Show"));
                menu.add(&TextEntry::of("quit", "Quit"));
                if let Some(id) = menu.popup(position) {
                    if id.0 == "show" {
                        let _ = tx.send(TrayMsg::Show);
                    } else if id.0 == "quit" {
                        let _ = tx.send(TrayMsg::Quit);
                        return;
                    }
                }
            }
            Ok(_) => {}
            Err(_) => return,
        }
    }
}

fn load_app_icon() -> Icon {
    let image = image::load_from_memory(APP_ICON).expect("embedded app icon should be a valid PNG").into_rgba8();
    let (width, height) = image.dimensions();
    Icon::from_rgba(image.into_raw(), width, height).expect("embedded app icon should be valid RGBA")
}
