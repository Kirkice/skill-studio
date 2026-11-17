use notify::{EventKind, RecursiveMode, Watcher};
use std::{path::{Path, PathBuf}, sync::mpsc, time::Duration};

fn relevant(root: &Path, event: &notify::Event) -> bool {
    if event.need_rescan() { return true; }
    if matches!(event.kind, EventKind::Access(_)) { return false; }
    event.paths.is_empty() || event.paths.iter().any(|path| {
        path.strip_prefix(root).map_or(true, |relative| !relative.components().any(|part| {
            matches!(part.as_os_str().to_str(), Some(".git" | "node_modules" | "target" | ".venv" | "dist"))
        }))
    })
}

pub fn start(root: &Path) -> Result<(notify::RecommendedWatcher, async_channel::Receiver<()>), String> {
    let root: PathBuf = root.into();
    let watched_root = root.clone();
    let (tx, rx) = mpsc::sync_channel(1024);
    let (events, receiver) = async_channel::bounded(1);
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if event.as_ref().is_ok_and(|event| !relevant(&root, event)) { return; }
        // If the queue is full, it already contains an event that forces a full rescan.
        let _ = tx.try_send(());
    }).map_err(|e| e.to_string())?;
    watcher.watch(&watched_root, RecursiveMode::Recursive).map_err(|e| e.to_string())?;
    std::thread::spawn(move || {
        while rx.recv().is_ok() {
            std::thread::sleep(Duration::from_millis(150));
            while rx.try_recv().is_ok() {}
            if events.is_closed() { break; }
            let _ = events.try_send(());
        }
    });
    Ok((watcher, receiver))
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::{event::{AccessKind, ModifyKind}, Event};

    #[test]
    fn filters_ignored_directories_and_read_events() {
        let root = Path::new("workspace");
        let event = |kind, path: &str| Event::new(kind).add_path(root.join(path));
        assert!(!relevant(root, &event(EventKind::Access(AccessKind::Any), "skill/SKILL.md")));
        assert!(!relevant(root, &event(EventKind::Modify(ModifyKind::Any), "node_modules/pkg/file.js")));
        assert!(relevant(root, &event(EventKind::Modify(ModifyKind::Any), "skill/SKILL.md")));
        assert!(relevant(root, &event(EventKind::Modify(ModifyKind::Any), ".gitignore")));
    }

    #[test]
    fn rescan_flag_overrides_filters() {
        let root = Path::new("workspace");
        let event = Event::new(EventKind::Access(AccessKind::Any)).add_path(root.join(".git/config")).set_flag(notify::event::Flag::Rescan);
        assert!(relevant(root, &event));
    }
}
