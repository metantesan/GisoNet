use crate::state::{GisoNet, InputField};
use gpui::prelude::*;
use gpui::*;

fn bg() -> Rgba {
    rgb(0x1e1e2e)
}
fn fg() -> Rgba {
    rgb(0xcdd6f4)
}
fn green() -> Rgba {
    rgb(0x4a_de_80)
}
fn red() -> Rgba {
    rgb(0xf3_8b_a8)
}
fn surface() -> Rgba {
    rgb(0x31_35_3e)
}
fn overlay() -> Rgba {
    rgb(0x45_47_5a)
}
fn muted() -> Rgba {
    rgb(0xa6_ad_bb)
}
fn blue() -> Rgba {
    rgb(0x89_b4_fa)
}
fn mauve() -> Rgba {
    rgb(0xcba6f7)
}
fn peach() -> Rgba {
    rgb(0xfab387)
}

fn card() -> gpui::Div {
    div().flex().flex_col().gap(px(6.0)).bg(surface()).rounded(px(10.0)).p(px(10.0))
}

fn section_title(text: &str) -> impl IntoElement {
    div().child(SharedString::from(text.to_owned())).text_size(px(14.0))
}

fn hint(text: &str) -> impl IntoElement {
    div().child(SharedString::from(text.to_owned())).text_color(muted())
}

impl Render for GisoNet {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.check_tray(cx);

        let status_color = if self.daemon_connected { green() } else { red() };
        let status_label = if self.daemon_connected {
            SharedString::from("daemon connected")
        } else {
            SharedString::from("daemon offline")
        };
        let route_count =
            SharedString::from(format!("{} route{}", self.routes.len(), if self.routes.len() == 1 { "" } else { "s" }));

        div()
            .id("app")
            .flex()
            .flex_col()
            .size_full()
            .overflow_y_scroll()
            .p(px(12.0))
            .gap(px(6.0))
            .bg(bg())
            .text_color(fg())
            .font_family("monospace")
            .text_size(px(13.0))
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
                this.handle_key(event, cx);
            }))
            .child(header(self, status_color, status_label, cx))
            .child(route_section(self, cx))
            .child(resolver_section(self, cx))
            .child(routes_section(self, route_count))
    }
}

// ── Header ──

fn header(_this: &mut GisoNet, status_color: Rgba, status_label: SharedString, cx: &mut Context<GisoNet>) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .bg(surface())
        .rounded(px(10.0))
        .p(px(10.0))
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .child(div().child(SharedString::from("GisoNet")).text_color(green()).text_size(px(18.0)))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(6.0))
                        .child(div().size(px(7.0)).rounded(px(3.0)).bg(status_color))
                        .child(div().child(status_label).text_color(status_color))
                        .child(btn(
                            SharedString::from("Start daemon"),
                            green(),
                            cx.listener(|this, _, _window, cx| {
                                this.run_daemon_with_sudo();
                                cx.notify();
                            }),
                        ))
                        .child(btn(
                            SharedString::from("Stop daemon"),
                            red(),
                            cx.listener(|this, _, _window, cx| {
                                this.stop_daemon();
                                cx.notify();
                            }),
                        )),
                ),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(div().child(SharedString::from("Browser")).text_size(px(10.0)))
                .child(div().child(SharedString::from("->")).text_color(muted()).text_size(px(10.0)))
                .child(div().child(SharedString::from("DNS IP")).text_color(blue()).text_size(px(10.0)))
                .child(div().child(SharedString::from("->")).text_color(muted()).text_size(px(10.0)))
                .child(div().child(SharedString::from("HTTPS proxy")).text_color(mauve()).text_size(px(10.0)))
                .child(div().child(SharedString::from("->")).text_color(muted()).text_size(px(10.0)))
                .child(div().child(SharedString::from("endpoint")).text_color(peach()).text_size(px(10.0))),
        )
}

// ── Route section ──

fn route_section(this: &mut GisoNet, cx: &mut Context<GisoNet>) -> impl IntoElement {
    card()
        .child(section_title("Route"))
        .child(hint("Domain → DNS IP → Endpoint"))
        .child(this.input_row(
            SharedString::from("Domain"),
            &this.domain,
            SharedString::from("myapp.example.com"),
            InputField::Domain,
            cx,
        ))
        .child(this.input_row(
            SharedString::from("DNS IP"),
            &this.ip,
            SharedString::from("127.0.0.1"),
            InputField::Ip,
            cx,
        ))
        .child(this.input_row(
            SharedString::from("Path"),
            &this.path,
            SharedString::from("/ or /api"),
            InputField::Path,
            cx,
        ))
        .child(this.input_row(
            SharedString::from("Endpoint"),
            &this.upstream,
            SharedString::from("http://127.0.0.1:3000"),
            InputField::Upstream,
            cx,
        ))
        .child(
            div()
                .flex()
                .flex_row()
                .gap(px(8.0))
                .child(btn(
                    SharedString::from("Add"),
                    green(),
                    cx.listener(|this, _, _window, cx| {
                        this.add_route();
                        cx.notify();
                    }),
                ))
                .child(btn(
                    SharedString::from("Remove"),
                    red(),
                    cx.listener(|this, _, _window, cx| {
                        this.remove_route();
                        cx.notify();
                    }),
                )),
        )
}

// ── Resolver section ──

fn resolver_section(this: &mut GisoNet, cx: &mut Context<GisoNet>) -> impl IntoElement {
    card()
        .child(section_title("DNS forwarding"))
        .child(hint("Non-matched domains forwarded to upstream resolver."))
        .child(this.input_row(
            SharedString::from("Resolver"),
            &this.resolver,
            SharedString::from("1.1.1.1:53"),
            InputField::Resolver,
            cx,
        ))
        .child(
            div()
                .flex()
                .flex_row()
                .gap(px(6.0))
                .child(btn(
                    SharedString::from("Set"),
                    blue(),
                    cx.listener(|this, _, _window, cx| {
                        this.set_resolver();
                        cx.notify();
                    }),
                ))
                .child(btn(
                    SharedString::from("Run daemon (sudo)"),
                    overlay(),
                    cx.listener(|this, _, _window, cx| {
                        this.run_daemon_with_sudo();
                        cx.notify();
                    }),
                )),
        )
}

// ── Routes list section ──

fn routes_section(this: &GisoNet, count: SharedString) -> impl IntoElement {
    card()
        .child(
            div()
                .flex()
                .flex_row()
                .justify_between()
                .child(section_title("Configured routes"))
                .child(div().child(count).text_color(muted())),
        )
        .child(hint("To remove a route, enter its domain and path above, then click Remove."))
        .child(div().flex().flex_col().gap(px(6.0)).children(this.routes.iter().map(|r| {
            let domain = SharedString::from(r.domain.clone());
            let dns = SharedString::from(format!("DNS  {}", r.ip));
            let proxy =
                SharedString::from(format!("Proxy {}  {}", if r.path.is_empty() { "/" } else { &r.path }, r.upstream));

            div()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .bg(overlay())
                .rounded(px(8.0))
                .p(px(8.0))
                .child(div().child(domain).text_color(green()))
                .child(div().child(dns).text_color(blue()))
                .child(div().child(proxy).text_color(peach()))
        })))
}

// ── Shared widgets ──

impl GisoNet {
    fn input_row(
        &self,
        label_text: SharedString,
        value: &str,
        placeholder: SharedString,
        field: InputField,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let is_active = self.active == Some(field);
        let bg = if is_active { overlay() } else { surface() };
        let border = if is_active { green() } else { overlay() };
        let text_color = if value.is_empty() { muted() } else { fg() };
        let display = if value.is_empty() { placeholder } else { SharedString::from(value.to_string()) };

        let field_id = field;
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(6.0))
            .child(div().child(label_text).text_size(px(11.0)).w(px(70.0)))
            .child(
                div()
                    .flex_1()
                    .px(px(8.0))
                    .py(px(4.0))
                    .rounded(px(4.0))
                    .bg(bg)
                    .border_1()
                    .border_color(border)
                    .cursor_text()
                    .child(div().child(display).text_color(text_color))
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(move |this, _, _window, cx| {
                            this.active = Some(field_id);
                            cx.notify();
                        }),
                    ),
            )
    }
}

fn btn(
    text: SharedString,
    color: Rgba,
    handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(ElementId::Name(text.clone()))
        .px(px(10.0))
        .py(px(4.0))
        .rounded(px(5.0))
        .bg(color)
        .text_color(bg())
        .text_size(px(11.0))
        .cursor_pointer()
        .child(text)
        .on_click(handler)
}
