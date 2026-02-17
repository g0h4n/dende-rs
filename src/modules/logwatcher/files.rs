use std::{
    collections::HashMap,
    fs::File,
    io::{self, BufRead, BufReader, Seek, SeekFrom},
    path::{Path, PathBuf},
};
use walkdir::WalkDir;
use anyhow::Result;

use crate::{utils::date::timestamp, Matcher};
use crate::notifiers::Notifier;
use log::{info,trace,error};

/// Per-file tailing state (byte offsets and line counters).
pub struct TailState {
    pub offsets: HashMap<PathBuf, u64>,
    pub line_nums: HashMap<PathBuf, u64>,
}
impl TailState {
    pub fn new() -> Self {
        Self { offsets: HashMap::new(), line_nums: HashMap::new() }
    }
}

/// Discover initial files and either read them fully or start tailing from EOF.
pub fn initialize_files(
    folder: &std::path::Path,
    recursive: bool,
    read_existing: bool,
    state: &mut TailState,
    matcher: &Matcher,
    notifier: &Notifier,
    only_message: bool
) -> Result<()> {
    // Support both a single file and a directory
    if folder.is_file() {
        if read_existing {
            read_new_lines(folder, state, matcher, notifier, true, only_message)?;
        } else {
            let len = std::fs::metadata(folder).map(|m| m.len()).unwrap_or(0);
            state.offsets.insert(folder.to_path_buf(), len);
            state.line_nums.insert(folder.to_path_buf(), 0);
        }
        return Ok(());
    }

    let walker = if recursive {
        WalkDir::new(folder).into_iter()
    } else {
        WalkDir::new(folder).max_depth(1).into_iter()
    };

    for entry in walker.filter_map(Result::ok).filter(|e| e.file_type().is_file()) {
        let path = entry.into_path();
        if read_existing {
            read_new_lines(&path, state, matcher, notifier, true, only_message)?;
        } else {
            let len = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            state.offsets.insert(path.clone(), len);
            state.line_nums.insert(path, 0);
        }
    }
    Ok(())
}

/// Read new lines from a file since last known offset; optionally from start.
pub fn read_new_lines(
    path: &Path,
    state: &mut TailState,
    matcher: &Matcher,
    notifier: &Notifier,
    from_scratch: bool,
    only_message: bool,
) -> io::Result<()> {
    let mut f = match File::open(path) {
        Ok(f) => f,
        Err(e) => {
            // File may be gone due to rotation.
            if e.kind() != io::ErrorKind::NotFound {
                error!("Open error {}: {e}", path.display());
            }
            return Ok(());
        }
    };

    let mut last_pos = if from_scratch { 0 } else { *state.offsets.get(path).unwrap_or(&0) };
    let meta_len = f.metadata().map(|m| m.len()).unwrap_or(0);
    if meta_len < last_pos { last_pos = 0; } // rotation/truncation detected

    f.seek(SeekFrom::Start(last_pos))?;
    let reader = BufReader::new(f);

    let mut byte_pos = last_pos;
    let mut line_no = *state.line_nums.get(path).unwrap_or(&0);

    for line_res in reader.lines() {
        let line = match line_res {
            Ok(s) => s,
            Err(e) => {
                error!("Read line error {}: {e}", path.display());
                break;
            }
        };
        line_no += 1;
        byte_pos += (line.len() + 1) as u64; // +1 ~ "\n" (approx, works for UNIX logs)
        if matcher.matches(&line) {

            info!("File {:?} match for {:?}", &path, &matcher);
            trace!("Only message: {:?} ", &only_message);

            let _txt = if only_message {
                format!("{}",line)
            } else {
                format!(
                "!dende-rs::log-watcher::matched!\n\nDate: {}\nFilename and line: {}:{}\nContent matched:\n\n{}",
                timestamp(),
                path.display(),
                line_no,
                line
                )
            };
            let _html = format!(
                "<b>!dende-rs::log-watcher::matched!</b>\n\n<i>Date:</i> <b>{}</b>\n<i>Filename and line:</i> <b>{}:{}</b>\n<i>Content matched:</i>\n\n{}",
                timestamp(),
                path.display(),
                line_no,
                line
            );

            trace!("\n{_txt}\n");
            notifier.notify(&_txt);
        }
    }

    state.offsets.insert(path.to_path_buf(), meta_len.max(byte_pos));
    state.line_nums.insert(path.to_path_buf(), line_no);
    Ok(())
}