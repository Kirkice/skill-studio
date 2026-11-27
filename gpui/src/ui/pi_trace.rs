use super::{controls::*, icons::Icon, theme::rgb, theme::*};
use gpui::*;
use serde_json::Value;
use std::{
    net::UdpSocket,
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum State {
    Running,
    Done,
    Failed,
    Interrupted,
    Skipped,
}
impl State {
    pub(crate) fn color(self) -> u32 {
        match self {
            Self::Running => 0x4ea1ff,
            Self::Done => 0x8acf97,
            Self::Failed => 0xdc8c95,
            Self::Interrupted | Self::Skipped => 0xc8a479,
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Running => "● Running",
            Self::Done => "✓ Observed",
            Self::Failed => "× Failed",
            Self::Interrupted => "– Interrupted",
            Self::Skipped => "– Skipped",
        }
    }
}
struct Call {
    id: String,
    name: String,
    path: Option<String>,
    state: State,
    ms: Option<f64>,
}
pub(crate) struct PiTraceView {
    bridge: Option<tempfile::TempDir>,
    details: Option<WindowHandle<TraceDetails>>,
    reader: Option<Task<()>>,
    command: String,
    status: String,
    calls: Vec<Call>,
    graph_highlights_hidden: bool,
    stages: std::collections::HashMap<(String, String), State>,
    tokens: u64,
    telemetry: Vec<String>,
    metrics: std::collections::HashMap<
        (String, String),
        (
            Option<std::time::Instant>,
            u64,
            Option<u64>,
            Option<(u64, u64)>,
        ),
    >,
}
impl PiTraceView {
    pub fn new() -> Self {
        Self {
            bridge: None,
            details: None,
            reader: None,
            command: String::new(),
            status: "Debug off".into(),
            calls: vec![],
            stages: Default::default(),
            graph_highlights_hidden: false,
            tokens: 0,
            telemetry: vec![],
            metrics: Default::default(),
        }
    }
    pub fn active(&self) -> bool {
        self.bridge.is_some()
    }
    pub(crate) fn paths(&self) -> std::collections::HashMap<String, State> {
        let mut paths = std::collections::HashMap::new();
        if self.graph_highlights_hidden {
            return paths;
        }
        for call in &self.calls {
            if let Some(path) = &call.path {
                let state = paths.entry(path.clone()).or_insert(call.state);
                if *state != State::Running {
                    *state = call.state;
                }
            }
        }
        paths
    }
    pub(super) fn graph_stages(&self) -> std::collections::HashMap<(String, String), State> {
        if self.graph_highlights_hidden {
            Default::default()
        } else {
            self.stages.clone()
        }
    }
    pub(super) fn clear_graph_highlights(&mut self, cx: &mut Context<Self>) {
        self.graph_highlights_hidden = true;
        cx.notify();
    }
    pub fn start(&mut self, root: PathBuf, launcher: &str, cx: &mut Context<Self>) {
        if self.active() {
            return;
        }
        match Self::prepare(&root, launcher) {
            Ok((dir, socket, command, token)) => {
                self.bridge = Some(dir);
                self.metrics.clear();
                self.telemetry.clear();
                self.command = command;
                self.calls.clear();
                self.stages.clear();
                self.tokens = 0;
                self.status = "Waiting for Pi · paste the copied command into the terminal".into();
                self.reader = Some(cx.spawn(async move |this, cx| {
                    let mut packet = [0u8; 8192];
                    loop {
                        let mut events = Vec::new();
                        for _ in 0..128 {
                            let Ok(n) = socket.recv(&mut packet) else {
                                break;
                            };
                            if let Ok(value) = serde_json::from_slice::<Value>(&packet[..n]) {
                                if value["key"].as_str() == Some(&token) {
                                    events.push(value);
                                }
                            }
                        }
                        if !events.is_empty()
                            && this
                                .update(cx, |this, cx| {
                                    for value in events {
                                        this.event(&root, value);
                                    }
                                    cx.notify();
                                })
                                .is_err()
                        {
                            break;
                        }
                        cx.background_executor()
                            .timer(Duration::from_millis(40))
                            .await;
                    }
                }));
            }
            Err(e) => self.status = format!("Could not start Debug: {e}"),
        }
        cx.notify();
    }
    fn prepare(
        root: &Path,
        launcher: &str,
    ) -> std::io::Result<(tempfile::TempDir, UdpSocket, String, String)> {
        let socket = UdpSocket::bind((std::net::Ipv4Addr::LOCALHOST, 0))?;
        socket.set_nonblocking(true)?;
        let port = socket.local_addr()?.port();
        let dir = tempfile::tempdir()?;
        let token = dir
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let script = dir.path().join("skillrider-trace.ts");
        std::fs::write(
            &script,
            format!(
                "const port = {port};\nconst key = {};\nconst marker = {};\n{}",
                serde_json::to_string(&token).unwrap(),
                serde_json::to_string(&script.to_string_lossy()).unwrap(),
                include_str!("pi_trace_bridge.ts")
            ),
        )?;
        let launcher = if launcher.trim().is_empty() {
            "pi"
        } else {
            launcher
        };
        let command = if cfg!(windows) {
            let quote = |s: &str| format!("'{}'", s.replace('\'', "''"));
            format!(
                "Set-Location -LiteralPath {}; & {} -e {}",
                quote(&crate::files::shell_display_path(root)),
                quote(launcher),
                quote(&script.to_string_lossy())
            )
        } else {
            let quote = |s: &str| format!("'{}'", s.replace('\'', "'\\''"));
            format!(
                "cd {} && {} -e {}",
                quote(&root.to_string_lossy()),
                quote(launcher),
                quote(&script.to_string_lossy())
            )
        };
        Ok((dir, socket, command, token))
    }
    pub fn copy_command(&self, cx: &mut App) {
        if self.active() {
            cx.write_to_clipboard(ClipboardItem::new_string(self.command.clone()));
        }
    }
    pub fn stop(&mut self, cx: &mut Context<Self>) {
        self.reader = None;
        self.bridge = None;
        self.command.clear();
        self.finish_pending();
        self.status = "Debug off · terminal left running".into();
        cx.notify();
    }
    fn finish_pending(&mut self) {
        for m in self.metrics.values_mut() {
            if let Some(t) = m.0.take() {
                m.1 += t.elapsed().as_millis() as u64;
            }
        }
        for state in self.stages.values_mut() {
            if *state == State::Running {
                *state = State::Interrupted;
            }
        }
        for call in &mut self.calls {
            if call.state == State::Running {
                call.state = State::Interrupted;
            }
        }
    }
    fn note(&mut self, line: String) {
        if self.telemetry.len() >= 1000 {
            self.telemetry.remove(0);
        }
        self.telemetry.push(line);
    }
    pub(super) fn metric_labels(&self) -> std::collections::HashMap<(String, String), String> {
        if self.graph_highlights_hidden {
            return Default::default();
        }
        let mut labels = std::collections::HashMap::new();
        for (key, m) in &self.metrics {
            let ms = m.1 + m.0.map_or(0, |t| t.elapsed().as_millis() as u64);
            labels.insert(
                key.clone(),
                format!(
                    "{:.2} s\n{}\n{}",
                    ms as f64 / 1000.,
                    m.2.map_or("—".into(), |v| v.to_string()),
                    m.3.map_or("—".into(), |(n, w)| format!("≈{:.3} / {:.3} MB", n as f64 * 0.000004, w as f64 * 0.000004))
                ),
            );
        }
        let mut files = std::collections::HashMap::<String, (usize, f64)>::new();
        for call in &self.calls {
            if call.name != "skillrider_stage"
                && let Some(path) = &call.path
            {
                let m = files.entry(path.clone()).or_default();
                m.0 += 1;
                m.1 += call.ms.unwrap_or(0.);
            }
        }
        for (path, (count, ms)) in files {
            labels.insert(
                (path, String::new()),
                format!("{ms:.0} ms ({count}×)\n—\n—"),
            );
        }
        labels
    }
    fn event(&mut self, root: &Path, value: Value) {
        let s = |k: &str| value[k].as_str().unwrap_or_default();
        match s("type") {
            "ready" => self.status = "Pi terminal connected".into(),
            "agent_start" => {
                self.metrics.clear();
                self.telemetry.clear();
                self.graph_highlights_hidden = false;
                self.calls.clear();
                self.stages.clear();
                self.tokens = 0;
                self.status = "Pi running".into();
            }
            "agent_settled" => {
                self.finish_pending();
                self.status = "Run finished · listening".into();
            }
            "session_shutdown" => {
                self.finish_pending();
                self.status = "Pi exited · waiting for terminal".into();
            }
            "stage" => {
                let state = match s("state") {
                    "running" => State::Running,
                    "done" => State::Done,
                    "failed" => State::Failed,
                    "skipped" => State::Skipped,
                    _ => return,
                };
                if let Some(path) = relative(root, s("cwd"), s("path")) {
                    let key = (path, s("heading").into());
                    let metric = self.metrics.entry(key.clone()).or_default();
                    if state == State::Running {
                        metric.0.get_or_insert_with(std::time::Instant::now);
                    } else if let Some(start) = metric.0.take() {
                        metric.1 += start.elapsed().as_millis() as u64;
                    }
                    self.stages.insert(key, state);
                    self.note(format!(
                        "{} · {} · explicit stage report",
                        state.label(),
                        s("heading")
                    ));
                }
            }
            "tool_execution_start" => {
                if self.calls.len() >= 1000 {
                    self.calls.remove(0);
                }
                self.calls.push(Call {
                    id: s("id").into(),
                    name: s("name").into(),
                    path: relative(root, s("cwd"), s("path")),
                    state: State::Running,
                    ms: None,
                });
            }
            "tool_execution_end" => {
                if let Some(call) = self.calls.iter_mut().rev().find(|call| call.id == s("id")) {
                    call.state = if value["failed"].as_bool() == Some(true) {
                        State::Failed
                    } else {
                        State::Done
                    };
                    call.ms = value["ms"].as_f64();
                    if !s("error").is_empty() {
                        let msg = format!("{} · {}", call.name, s("error"));
                        self.note(msg);
                    }
                }
            }
            "activity" => self.note(s("label").into()),
            "message_end" => {
                let context = value["context"].as_u64().zip(value["window"].as_u64());
                let mut active = self.stages.iter().filter(|(_, v)| **v == State::Running);
                if let Some((key, _)) = active.next()
                    && active.next().is_none()
                {
                    let m = self.metrics.entry(key.clone()).or_default();
                    if let Some(n) = value["tokens"].as_u64() {
                        m.2 = Some(m.2.unwrap_or(0).saturating_add(n));
                    }
                    if let Some(c) = context
                        && m.3.is_none_or(|old| c.0 > old.0)
                    {
                        m.3 = Some(c);
                    }
                }
                self.note(format!(
                    "Model {} · in {} / out {} / cache {} · context {} · cost {} · {}",
                    s("model"),
                    value["input"],
                    value["output"],
                    value["cache"],
                    context.map_or("—".into(), |(a, b)| format!("≈{a}/{b}")),
                    value["cost"],
                    s("error")
                ));
                self.tokens = self
                    .tokens
                    .saturating_add(value["tokens"].as_u64().unwrap_or(0))
            }
            _ => {}
        }
    }
}
fn relative(root: &Path, cwd: &str, path: &str) -> Option<String> {
    if path.is_empty() {
        return None;
    }
    let file = Path::new(path);
    let absolute = if file.is_absolute() {
        file.to_path_buf()
    } else {
        Path::new(cwd).join(file)
    };
    let root = crate::terminal::shell_cwd(root);
    let mut parts = Vec::new();
    for component in absolute.strip_prefix(root).ok()?.components() {
        match component {
            std::path::Component::Normal(p) => parts.push(p.to_str()?),
            std::path::Component::CurDir => {}
            _ => return None,
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}
struct TraceDetails {
    trace: WeakEntity<PiTraceView>,
    _observe: Subscription,
}

impl Render for TraceDetails {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(trace) = self.trace.upgrade() else {
            return div().size_full().child(crate::i18n::text("Pi Trace is unavailable"));
        };
        let trace = trace.read(cx);
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(BG))
            .text_color(rgb(TEXT))
            .font_family(UI_FONT)
            .child(
                bar()
                    .bg(rgb(PANEL))
                    .child(caption(crate::i18n::text("Pi Trace")))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .truncate()
                            .text_size(px(12.))
                            .child(trace.status.clone()),
                    )
                    .child(caption(format!("{} tokens", trace.tokens))),
            )
            .child(trace.rows())
            .child(
                div()
                    .id("pi-telemetry")
                    .flex_1()
                    .min_h(px(0.))
                    .overflow_y_scroll()
                    .p(px(8.))
                    .border_t_1()
                    .border_color(rgb(BORDER))
                    .child(caption(crate::i18n::text("Run telemetry")))
                    .children(
                        trace
                            .telemetry
                            .iter()
                            .rev()
                            .map(|event| div().text_size(px(11.)).child(event.clone())),
                    ),
            )
    }
}

impl PiTraceView {
    pub(crate) fn close_details(&mut self, cx: &mut App) {
        if let Some(window) = self.details.take() {
            let _ = window.update(cx, |_, window, _| window.remove_window());
        }
    }

    pub(super) fn show_details(&mut self, cx: &mut Context<Self>) {
        let weak = cx.entity().downgrade();
        cx.defer(move |cx| {
            let Some(trace) = weak.upgrade() else {
                return;
            };
            if let Some(window) = trace.read(cx).details {
                if window
                    .update(cx, |_, window, _| window.activate_window())
                    .is_ok()
                {
                    return;
                }
            }
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(720.), px(440.)),
                    cx,
                ))),
                window_min_size: Some(size(px(340.), px(220.))),
                ..Default::default()
            };
            let observed = trace.clone();
            let details = cx
                .open_window(options, move |window, cx| {
                    window.set_window_title("Pi Trace");
                    cx.new(|cx| TraceDetails {
                        trace: observed.downgrade(),
                        _observe: cx.observe(&observed, |_, _, cx| cx.notify()),
                    })
                })
                .ok();
            trace.update(cx, |trace, _| trace.details = details);
        });
    }

    fn rows(&self) -> Stateful<Div> {
        let mut rows = div()
            .id("pi-observed-calls")
            .flex_1()
            .min_h(px(0.))
            .overflow_y_scroll()
            .p(px(6.));
        for call in self.calls.iter().rev() {
            rows = rows.child(
                div()
                    .flex()
                    .gap(px(8.))
                    .px(px(6.))
                    .min_w(px(0.))
                    .text_size(px(11.))
                    .child(
                        div()
                            .flex_none()
                            .text_color(rgb(call.state.color()))
                            .child(call.state.label()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .truncate()
                            .child(match &call.path {
                                Some(path) => format!("{} · {path}", call.name),
                                None => call.name.clone(),
                            }),
                    )
                    .children(call.ms.map(|ms| caption(format!("{ms:.0} ms")))),
            );
        }
        if self.calls.is_empty() {
            rows = rows.child(
                div()
                    .p(px(12.))
                    .text_color(rgb(MUTED))
                    .child(crate::i18n::text("No tool calls recorded yet.")),
            );
        }
        rows
    }
}

impl Render for PiTraceView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let status_color = if self.status.starts_with("Could not") {
            0xdc8c95
        } else if !self.active() {
            MUTED
        } else if self.status.starts_with("Waiting") || self.status.starts_with("Pi exited") {
            WARNING
        } else if self.status == "Pi running" {
            ACCENT
        } else {
            GREEN
        };
        div()
            .size_full()
            .min_h(px(0.))
            .overflow_hidden()
            .flex()
            .flex_col()
            .bg(rgb(PANEL))
            .border_t_1()
            .border_color(rgb(BORDER))
            .child(
                div()
                    .size_full()
                    .px(px(8.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        icon_label(
                            "pi-trace-details",
                            Icon::ExternalWindow,
                            "Pi Trace",
                            false,
                            true,
                        )
                        .h(px(24.))
                        .border_1()
                        .border_color(rgb(BORDER))
                        .bg(rgb(RAISED))
                        .tooltip(|_, cx| cx.new(|_| Hint("Open Pi Trace details".into())).into())
                        .on_click(cx.listener(|this, _, _, cx| this.show_details(cx))),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .child(
                                div()
                                    .size(px(6.))
                                    .flex_none()
                                    .rounded_full()
                                    .bg(rgb(status_color)),
                            )
                            .child(
                                div()
                                    .id("pi-trace-status")
                                    .min_w(px(0.))
                                    .truncate()
                                    .text_size(px(11.))
                                    .text_color(rgb(status_color))
                                    .child(if self.status.starts_with("Waiting for Pi") {
                                        "Waiting for Pi".to_string()
                                    } else {
                                        self.status.clone()
                                    })
                                    .tooltip({
                                        let status: SharedString = self.status.clone().into();
                                        move |_, cx| cx.new(|_| Hint(status.clone())).into()
                                    }),
                            ),
                    )
                    .child(caption(format!("{} tokens", self.tokens)).flex_none())
                    .children(self.active().then(|| {
                        icon_label("pi-copy-attach", Icon::Copy, "Copy", false, true)
                            .h(px(24.))
                            .border_1()
                            .border_color(rgb(BORDER))
                            .bg(rgb(RAISED))
                            .aria_label("Copy Pi command")
                            .tooltip(|_, cx| cx.new(|_| Hint("Copy Pi command".into())).into())
                            .on_click(cx.listener(|this, _, _, cx| this.copy_command(cx)))
                    })),
            )
    }
}
