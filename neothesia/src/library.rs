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
    /// Músicas marcadas com ★
    pub favorites: Vec<PathBuf>,
    /// Últimas músicas tocadas, a mais recente primeiro (máx. `MAX_RECENT`)
    pub recent: Vec<PathBuf>,
    /// Pastas de gravações externas ligadas a uma música: (pasta, música).
    /// Os `.mid` dessas pastas são importados como gravações (ver
    /// `recording::import`) e não aparecem como músicas na biblioteca.
    pub links: Vec<(PathBuf, PathBuf)>,
}

pub const MAX_RECENT: usize = 20;

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
                    "fav" => lib.favorites.push(PathBuf::from(path)),
                    "recent" => lib.recent.push(PathBuf::from(path)),
                    "link" => {
                        if let Some((folder, song)) = path.split_once('\t') {
                            lib.links.push((PathBuf::from(folder), PathBuf::from(song)));
                        }
                    }
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
        for f in &self.favorites {
            out.push_str(&format!("fav\t{}\n", f.display()));
        }
        for r in &self.recent {
            out.push_str(&format!("recent\t{}\n", r.display()));
        }
        for (folder, song) in &self.links {
            out.push_str(&format!("link\t{}\t{}\n", folder.display(), song.display()));
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
        found.retain(|p| !self.links.iter().any(|(folder, _)| p.starts_with(folder)));
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

    /// Liga uma pasta de gravações externas a uma música (substitui uma ligação
    /// anterior da mesma pasta).
    pub fn link_recordings(&mut self, folder: PathBuf, song: PathBuf) {
        self.links.retain(|(f, _)| f != &folder);
        self.links.push((folder, song));
        self.save();
        self.rescan();
    }

    pub fn unlink_recordings(&mut self, folder: &Path) {
        self.links.retain(|(f, _)| f != folder);
        self.save();
        self.rescan();
    }

    /// Pasta de gravações ligada a esta música, se houver.
    pub fn linked_folder(&self, song: &Path) -> Option<&Path> {
        self.links.iter().find(|(_, s)| s == song).map(|(f, _)| f.as_path())
    }

    pub fn is_favorite(&self, path: &Path) -> bool {
        self.favorites.iter().any(|f| f == path)
    }

    pub fn toggle_favorite(&mut self, path: &Path) {
        if let Some(i) = self.favorites.iter().position(|f| f == path) {
            self.favorites.remove(i);
        } else {
            self.favorites.push(path.to_path_buf());
        }
        self.save();
    }

    /// Regista que `path` foi tocada (chamado sempre que uma música começa).
    pub fn mark_played(&mut self, path: &Path) {
        self.recent.retain(|r| r != path);
        self.recent.insert(0, path.to_path_buf());
        self.recent.truncate(MAX_RECENT);
        self.save();
    }

    pub fn stop_playlist(&mut self) {
        self.queue_pos = None;
    }

    /// Título limpo e compositor a partir do nome do ficheiro, ex.:
    /// "Frédéric François Chopin_(1810 - 1849) - Waltz in C♯ Minor (1)"
    /// → ("Waltz in C♯ Minor", Some("Frédéric François Chopin")).
    pub fn pretty_title(path: &Path) -> (String, Option<String>) {
        pretty_title_of(&Self::display_name(path))
    }

    pub fn display_name(path: &Path) -> String {
        path.file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned())
    }
}

pub fn pretty_title_of(stem: &str) -> (String, Option<String>) {
    let stem = stem.replace('_', " ");
    // Tirar sufixos de cópia: " (1)", " (2)"…
    let mut s = stem.trim().to_owned();
    if let Some(i) = s.rfind(" (")
        && s.ends_with(')')
        && s[i + 2..s.len() - 1].chars().all(|c| c.is_ascii_digit())
    {
        s.truncate(i);
    }
    // Tirar grupos entre parênteses com algarismos (datas do compositor)
    let mut clean = String::with_capacity(s.len());
    let mut rest = s.as_str();
    while let Some(i) = rest.find('(') {
        let Some(j) = rest[i..].find(')') else { break };
        let inner = &rest[i + 1..i + j];
        clean.push_str(&rest[..i]);
        if !inner.chars().any(|c| c.is_ascii_digit()) {
            clean.push_str(&rest[i..=i + j]);
        }
        rest = &rest[i + j + 1..];
    }
    clean.push_str(rest);
    let s = clean.split_whitespace().collect::<Vec<_>>().join(" ");
    let Some((left, right)) = s.split_once(" - ") else {
        return (s.trim().to_owned(), None);
    };
    // Compositor: tirar datas entre parênteses "(1810 - 1849)"
    let mut composer = left.trim().to_owned();
    if let Some(i) = composer.find('(')
        && composer[i..].chars().any(|c| c.is_ascii_digit())
    {
        composer.truncate(i);
    }
    let composer = composer.split_whitespace().collect::<Vec<_>>().join(" ");
    let title = right.trim().to_owned();
    if composer.is_empty() || title.is_empty() {
        (s.trim().to_owned(), None)
    } else {
        (title, Some(composer))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn lib_with(queue: &[&str]) -> Library {
        Library {
            queue: queue.iter().map(PathBuf::from).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn advance_and_previous_walk_the_queue() {
        let mut lib = lib_with(&["a.mid", "b.mid", "c.mid"]);
        lib.queue_pos = Some(0);
        assert_eq!(lib.position_label().as_deref(), Some("1 de 3"));
        assert!(!lib.has_previous());
        assert_eq!(lib.advance(), Some(PathBuf::from("b.mid")));
        assert_eq!(lib.advance(), Some(PathBuf::from("c.mid")));
        assert!(!lib.has_next());
        assert_eq!(lib.previous(), Some(PathBuf::from("b.mid")));
        assert_eq!(lib.queue_pos, Some(1));
    }

    #[test]
    fn advance_past_end_stops_playlist() {
        let mut lib = lib_with(&["a.mid"]);
        lib.queue_pos = Some(0);
        assert_eq!(lib.advance(), None);
        assert_eq!(lib.queue_pos, None);
    }

    #[test]
    fn pretty_titles() {
        assert_eq!(
            pretty_title_of("Frédéric François Chopin_(1810 - 1849) - Waltz in C♯ Minor (1)"),
            ("Waltz in C♯ Minor".into(), Some("Frédéric François Chopin".into()))
        );
        assert_eq!(
            pretty_title_of("Piotr Iljicz Czajkowski - Dance of the Sugar Plum Fairy"),
            ("Dance of the Sugar Plum Fairy".into(), Some("Piotr Iljicz Czajkowski".into()))
        );
        assert_eq!(pretty_title_of("Interstellar"), ("Interstellar".into(), None));
        assert_eq!(
            pretty_title_of("Claude Debussy_(1862–1918) - Clair de Lune"),
            ("Clair de Lune".into(), Some("Claude Debussy".into()))
        );
    }

    #[test]
    fn search_matches_all_words_case_insensitive() {
        let s = LibSong::new(PathBuf::from("/x/Chopin - Nocturne Op.9.mid"));
        assert!(s.matches("chopin noct"));
        assert!(!s.matches("chopin waltz"));
    }
}
