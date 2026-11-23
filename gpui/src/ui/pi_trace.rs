use super::{controls::*, theme::rgb, theme::*};
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
}
impl State {
    pub(crate) fn color(self) -> u32 {
        match self {
            Self::Running => 0x4ea1ff,
            Self::Done => 0x8acf97,
            Self::Failed => 0xdc8c95,
            Self::Interrupted => 0xc8a479,
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Running => "● Running",
            Self::Done => "✓ Observed",
            Self::Failed => "× Failed",
            Self::Interrupted => "– Interrupted",
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
    pub(super) expanded: bool,
    reader: Option<Task<()>>,
    command: String,
    status: String,
    calls: Vec<Call>,
    tokens: u64,
}
impl PiTraceView {
    pub fn new() -> Self {
        Self {
            bridge: None,
            expanded: false,
            reader: None,
            command: String::new(),
            status: "Debug off".into(),
            calls: vec![],
            tokens: 0,
        }
    }
    pub fn active(&self) -> bool {
        self.bridge.is_some()
    }
    pub(crate) fn paths(&self) -> std::collections::HashMap<String, State> {
        let mut paths = std::collections::HashMap::new();
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
    pub fn start(&mut self, root: PathBuf, launcher: &str, cx: &mut Context<Self>) {
        if self.active() {
            return;
        }
        match Self::prepare(&root, launcher) {
            Ok((dir, socket, command, token)) => {
                self.bridge = Some(dir);
                self.command = command;
                self.calls.clear();
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
        for call in &mut self.calls {
            if call.state == State::Running {
                call.state = State::Interrupted;
            }
        }
    }
    fn event(&mut self, root: &Path, value: Value) {
        let s = |k: &str| value[k].as_str().unwrap_or_default();
        match s("type") {
            "ready" => self.status = "Pi terminal connected".into(),
            "agent_start" => {
                self.calls.clear();
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
                }
            }
            "message_end" => {
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
impl Render for PiTraceView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
                bar()
                    .h(px(32.))
                    .child(
                        button(
                            "pi-trace-expand",
                            if self.expanded {
                                "▾ Pi Trace"
                            } else {
                                "▸ Pi Trace"
                            },
                            self.expanded,
                            true,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.expanded = !this.expanded;
                            cx.notify();
                        })),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .truncate()
                            .text_size(px(11.))
                            .child(self.status.clone()),
                    )
                    .child(caption(format!("{} tokens", self.tokens)))
                    .children(self.active().then(|| {
                        button("pi-copy-attach", "Copy Pi command", false, true)
                            .on_click(cx.listener(|this, _, _, cx| this.copy_command(cx)))
                    })),
            )
            .children(self.expanded.then_some(rows))
    }
}
