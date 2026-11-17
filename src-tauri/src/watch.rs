use notify::{EventKind, RecursiveMode, Watcher};
use std::{path::Path, sync::mpsc, time::Duration};
use tauri::{AppHandle, Emitter};

// The watcher is owned by the workspace and dropped when the workspace changes.
pub fn start(root: &Path, app: AppHandle) -> Result<notify::RecommendedWatcher, String> {
    let watched_root = root.to_path_buf();
    let (tx, rx) = mpsc::sync_channel(1024);
    let overflow = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let lost = overflow.clone();
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if let Ok(e) = &event {
            if !e.need_rescan() && matches!(e.kind, EventKind::Access(_)) { return; }
            if !e.need_rescan() && !e.paths.is_empty() && e.paths.iter().all(|path| {
                path.strip_prefix(&watched_root).is_ok_and(|relative| relative.components().any(|part| {
                    matches!(part.as_os_str().to_str(), Some(".git" | "node_modules" | "target" | ".venv" | "dist"))
                }))
            }) { return; }
        }
        if tx.try_send(event).is_err() { lost.store(true, std::sync::atomic::Ordering::Relaxed); }
    }).map_err(|e| e.to_string())?;
    watcher.watch(root, RecursiveMode::Recursive).map_err(|e| e.to_string())?;
    let root = root.to_string_lossy().into_owned();
    std::thread::spawn(move || {
        while rx.recv().is_ok() {
            std::thread::sleep(Duration::from_millis(150));
            while rx.try_recv().is_ok() {}
            overflow.store(false, std::sync::atomic::Ordering::Relaxed);
            // Any event, error or overflow invalidates the entire index.
            let _ = app.emit("workspace-changed", &root);
        }
    });
    Ok(watcher)
}
