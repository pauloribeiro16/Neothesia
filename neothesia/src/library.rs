//! Biblioteca de músicas + playlist (fila), persistida em `library.txt`.
//!
//! Formato do ficheiro (uma entrada por linha):
//! ```text
//! folder<TAB>/caminho/para/pasta
//! queue<TAB>/caminho/para/musica.mid
//! ```

use std::path::{Path, PathBuf};

use crate::scene::menu_scene::is_midi_file;

const MAX_DEPTH: usize = 8;

#[derive(Debug, Clone)]
pub struct LibSong {
    pub path: PathBuf,
    pub title: String,
    /// Texto em minúsculas usado na pesquisa
    search_key: String,
}

impl LibSong {
    pub fn new(path: PathBuf) -> Self {
        let title = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());
        let search_key = title.to_lowercase();
        Self {
            path,
            title,
            search_key,
        }
    }

    pub fn matches(&self, query_lower: &str) -> bool {
        query_lower
            .split_whitespace()
            .all(|word| self.search_key.contains(word))
    }
}

#[derive(Default)]
pub struct Library {
    pub folders: Vec<PathBuf>,
    pub songs: Vec<LibSong>,
    pub queue: Vec<PathBuf>,
    /// Posição da playlist que está a tocar (None = a tocar fora da playlist)
    pub queue_pos: Option<usize>,
}

fn store_path() -> PathBuf {
    let base = std::env::var_os("HOME")
        .filter(|h| !h.is_empty())
        .map(PathBuf::from)
        .map(|h| h.join(".config").join("neothesia"))
        .or_else(|| {
            std::env::var_os("APPDATA")
                .map(PathBuf::from)
                .map(|p| p.join("neothesia"))
        })
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("library.txt")
}

impl Library {
    pub fn load() -> Self {
        let mut lib = Self::default();

        if let Ok(text) = std::fs::read_to_string(store_path()) {
            for line in text.lines() {
                let Some((kind, path)) = line.split_once('\t') else {
                    continue;
                };
                match kind {
                    "folder" => lib.folders.push(PathBuf::from(path)),
                    "queue" => lib.queue.push(PathBuf::from(path)),
                    _ => {}
                }
            }
        }

        lib.rescan();
        lib
    }

    pub fn save(&self) {
        let mut out = String::new();
        for f in &self.folders {
            out.push_str(&format!("folder\t{}\n", f.display()));
        }
        for q in &self.queue {
            out.push_str(&format!("queue\t{}\n", q.display()));
        }

        let path = store_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(e) = std::fs::write(&path, out) {
            log::error!("Failed to save library: {e}");
        }
    }

    pub fn rescan(&mut self) {
        let mut found = Vec::new();
        for folder in &self.folders {
            scan_dir(folder, 0, &mut found);
        }
        found.sort_by_key(|p| p.to_string_lossy().to_lowercase());
        found.dedup();
        self.songs = found.into_iter().map(LibSong::new).collect();
    }

    pub fn add_folder(&mut self, folder: PathBuf) {
        if !self.folders.contains(&folder) {
            self.folders.push(folder);
            self.save();
            self.rescan();
        }
    }

    pub fn remove_folder(&mut self, index: usize) {
        if index < self.folders.len() {
            self.folders.remove(index);
            self.save();
            self.rescan();
        }
    }

    pub fn enqueue(&mut self, path: PathBuf) {
        self.queue.push(path);
        self.save();
    }

    pub fn enqueue_many(&mut self, paths: impl IntoIterator<Item = PathBuf>) {
        self.queue.extend(paths);
        self.save();
    }

    pub fn clear_queue(&mut self) {
        self.queue.clear();
        self.queue_pos = None;
        self.save();
    }

    pub fn remove_from_queue(&mut self, index: usize) {
        if index >= self.queue.len() {
            return;
        }
        self.queue.remove(index);
        self.queue_pos = match self.queue_pos {
            Some(p) if p == index => None,
            Some(p) if p > index => Some(p - 1),
            other => other,
        };
        self.save();
    }

    pub fn move_in_queue(&mut self, index: usize, up: bool) {
        let other = if up {
            let Some(o) = index.checked_sub(1) else {
                return;
            };
            o
        } else {
            index + 1
        };
        if index >= self.queue.len() || other >= self.queue.len() {
            return;
        }
        self.queue.swap(index, other);
        if let Some(p) = self.queue_pos {
            if p == index {
                self.queue_pos = Some(other);
            } else if p == other {
                self.queue_pos = Some(index);
            }
        }
        self.save();
    }

    /// Avança para a próxima música da playlist (devolve o caminho).
    pub fn advance(&mut self) -> Option<PathBuf> {
        let next = self.queue_pos? + 1;
        match self.queue.get(next) {
            Some(path) => {
                self.queue_pos = Some(next);
                Some(path.clone())
            }
            None => {
                self.queue_pos = None;
                None
            }
        }
    }

    /// Volta à música anterior da playlist (devolve o caminho).
    pub fn previous(&mut self) -> Option<PathBuf> {
        let prev = self.queue_pos?.checked_sub(1)?;
        let path = self.queue.get(prev)?.clone();
        self.queue_pos = Some(prev);
        Some(path)
    }

    /// Há uma música seguinte na playlist?
    pub fn has_next(&self) -> bool {
        self.queue_pos.is_some_and(|p| p + 1 < self.queue.len())
    }

    /// Há uma música anterior na playlist?
    pub fn has_previous(&self) -> bool {
        self.queue_pos.is_some_and(|p| p > 0)
    }

    /// Texto "3 de 8" quando se está a tocar a playlist; `None` caso contrário.
    pub fn position_label(&self) -> Option<String> {
        self.queue_pos
            .map(|p| format!("{} de {}", p + 1, self.queue.len()))
    }

    pub fn stop_playlist(&mut self) {
        self.queue_pos = None;
    }

    pub fn display_name(path: &Path) -> String {
        path.file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned())
    }
}

fn scan_dir(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    if depth > MAX_DEPTH {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(ty) = entry.file_type() else { continue };
        if ty.is_dir() {
            scan_dir(&path, depth + 1, out);
        } else if is_midi_file(&path) {
            out.push(path);
        }
    }
}
