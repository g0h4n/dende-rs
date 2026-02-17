use notify::{recommended_watcher, Event, EventKind, RecursiveMode, Watcher};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use log::error;

use crate::modules::logwatcher::files::{initialize_files, read_new_lines, TailState};
use crate::Matcher;
use crate::notifiers::Notifier;

/// Handle a single filesystem event and read new lines if appropriate.
pub fn handle_event(
    event: Event,
    state: &mut TailState,
    matcher: &Matcher,
    notifier: &Notifier,
    watch_file_name: Option<&std::ffi::OsStr>,
    only_message: bool,
) {
    use notify::event::{CreateKind, ModifyKind, RenameMode};

    let mut process_path = |path: std::path::PathBuf, from_start: bool| {
        if let Some(name) = watch_file_name {
            if path.file_name() != Some(name) { return; }
        }
        if let Err(e) = read_new_lines(&path, state, matcher, notifier, from_start, only_message) {
            error!("FS read error {}: {}", path.display(), e);
        }
    };

    match event.kind {
        EventKind::Create(CreateKind::File) => {
            for path in event.paths { process_path(path, true); }
        }
        EventKind::Modify(ModifyKind::Data(_)) | EventKind::Modify(ModifyKind::Any) => {
            for path in event.paths { process_path(path, false); }
        }
        EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
            for path in event.paths { process_path(path, true); }
        }
        EventKind::Remove(_) => {
            for path in event.paths {
                if let Some(name) = watch_file_name {
                    if path.file_name() != Some(name) { continue; }
                }
                state.offsets.remove(&path);
                state.line_nums.remove(&path);
            }
        }
        _ => {}
    }
}

/// Spawn a watcher thread for a job. If a file path is provided, watch its parent
/// directory and filter events to that file name; otherwise watch the directory.
pub fn spawn_job_watcher(
    idx: usize,
    folder: std::path::PathBuf,
    recursive: bool,
    read_existing: bool,
    matcher: crate::Matcher,
    notifier: Notifier,
    only_message: bool,
) -> thread::JoinHandle<()> {
    thread::Builder::new()
        .name(format!("watcher-{}", idx))
        .spawn(move || {
            let mut state = TailState::new();

            // Initialize (reads existing content or sets offsets)
            if let Err(e) = initialize_files(&folder, recursive, read_existing, &mut state, &matcher, &notifier, only_message) {
                error!("[job {}] init error: {}", idx, e);
                return;
            }

            // Decide what to watch
            let watching_file = folder.is_file();
            let watch_root = if watching_file {
                folder.parent().unwrap_or(folder.as_path()).to_path_buf()
            } else {
                folder.clone()
            };
            let watch_name = if watching_file { folder.file_name().map(|s| s.to_owned()) } else { None };

            // Start watcher
            let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
            let mut watcher = match recommended_watcher(move |res| { let _ = tx.send(res); }) {
                Ok(w) => w,
                Err(e) => { error!("[job {}] watcher error: {}", idx, e); return; }
            };

            let mode = if watching_file { RecursiveMode::NonRecursive } else if recursive { RecursiveMode::Recursive } else { RecursiveMode::NonRecursive };
            if let Err(e) = watcher.watch(&watch_root, mode) {
                error!("[job {}] watch() error: {}", idx, e);
                return;
            }

            loop {
                match rx.recv_timeout(Duration::from_secs(1)) {
                    Ok(Ok(event)) => handle_event(event, &mut state, &matcher, &notifier, watch_name.as_deref(), only_message),
                    Ok(Err(err)) => error!("[job {}] event error: {}", idx, err),
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(e) => { error!("[job {}] channel error: {}", idx, e); break; }
                }
            }
        })
        .expect("spawn watcher thread")
}