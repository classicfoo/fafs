use crate::config::SearchType;
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::SystemTime,
};
use walkdir::WalkDir;

#[derive(Clone, Debug)]
pub struct SearchResult {
    pub name: String,
    pub path: PathBuf,
    pub is_folder: bool,
    pub modified: SystemTime,
}

pub fn scan(
    root: &Path,
    query: &str,
    search_type: SearchType,
    generation: u64,
    latest_generation: &AtomicU64,
) -> Result<Vec<SearchResult>, String> {
    if !root.is_dir() {
        return Err(format!(
            "Search directory does not exist: {}",
            root.display()
        ));
    }

    let keywords: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    let mut results = Vec::new();

    for entry in WalkDir::new(root).min_depth(1).follow_links(false) {
        if latest_generation.load(Ordering::Relaxed) != generation {
            return Ok(Vec::new());
        }

        let Ok(entry) = entry else {
            continue;
        };
        let is_folder = entry.file_type().is_dir();
        if search_type == SearchType::Folders && !is_folder {
            continue;
        }

        let name = entry.file_name().to_string_lossy().into_owned();
        let lowered = name.to_lowercase();
        if !keywords.iter().all(|keyword| lowered.contains(keyword)) {
            continue;
        }

        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        results.push(SearchResult {
            name,
            path: entry.into_path(),
            is_folder,
            modified: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
        });
    }

    results.sort_by(|left, right| right.modified.cmp(&left.modified));
    Ok(results)
}

pub fn next_generation(generation: &Arc<AtomicU64>) -> u64 {
    generation.fetch_add(1, Ordering::AcqRel) + 1
}
