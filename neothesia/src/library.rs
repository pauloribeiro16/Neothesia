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
    /// Título limpo (da pasta da música ou do nome do ficheiro)
    pub title: String,
    /// Autor, se conhecido (da pasta do autor ou do nome do ficheiro)
    pub composer: Option<String>,
    /// Chave normalizada do autor (ex.: "chopin"), para agrupar
    pub composer_key: Option<String>,
    /// Texto em minúsculas usado na pesquisa (título + autor)
    search_key: String,
}

impl LibSong {
    pub fn new(path: PathBuf) -> Self {
        Self::with_root(path, None)
    }

    /// Lê título e autor. Convenção de pastas (tem prioridade sobre o nome do ficheiro):
    /// - `<raiz>/<Autor>/<Música>/<ficheiro>.mid` → autor e título das pastas
    /// - `<raiz>/<Autor>/<ficheiro>.mid` → autor da pasta, título do ficheiro
    pub fn with_root(path: PathBuf, root: Option<&Path>) -> Self {
        let (mut title, mut composer) = Library::pretty_title(&path);
        if let Some(rel) = root.and_then(|r| path.strip_prefix(r).ok()) {
            let dirs: Vec<String> = rel
                .parent()
                .map(|p| p.iter().map(|c| c.to_string_lossy().into_owned()).collect())
                .unwrap_or_default();
            match dirs.as_slice() {
                [author, song, ..] => {
                    composer = Some(author.clone());
                    title = song.clone();
                }
                [author] => composer = Some(author.clone()),
                [] => {}
            }
        }
        if let Some((t, c)) = title_override(&path) {
            title = t;
            if c.is_some() {
                composer = c;
            }
        }
        let composer_key = composer.as_deref().map(composer_key);
        let search_key = format!("{} {}", title, composer.as_deref().unwrap_or("")).to_lowercase();
        Self {
            path,
            title,
            composer,
            composer_key,
            search_key,
        }
    }

    fn with_override(self) -> Self {
        // with_root já aplica o título escolhido; mantido para clareza do chamador
        self
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
    /// Títulos/autores escolhidos pelo utilizador: (música, título, autor)
    pub titles: Vec<(PathBuf, String, Option<String>)>,
    /// Leitura das pastas a decorrer em segundo plano (ver `rescan`/`poll_scan`)
    scan_rx: Option<std::sync::mpsc::Receiver<Vec<LibSong>>>,
}

pub const MAX_RECENT: usize = 20;

/// Títulos e autores escolhidos pelo utilizador, por música (caminho na forma
/// do disco). Têm prioridade sobre as pastas e o nome do ficheiro em TODA a app,
/// incluindo `Library::pretty_title`. Persistidos em `library.txt` (linhas `title`).
static TITLES: std::sync::RwLock<Vec<(PathBuf, String, Option<String>)>> =
    std::sync::RwLock::new(Vec::new());

fn title_override(path: &Path) -> Option<(String, Option<String>)> {
    let t = TITLES.read().ok()?;
    t.iter()
        .find(|(p, _, _)| p == path)
        .map(|(_, title, composer)| (title.clone(), composer.clone()))
}

/// Caminho na forma exata em que está no disco.
///
/// No macOS o mesmo nome pode estar escrito com "é" composto (U+00E9) ou
/// decomposto ("e" + U+0301). O disco aceita os dois, mas a comparação de
/// `PathBuf` é byte a byte: sem isto, a música da biblioteca e a das favoritas
/// ou gravações pareciam diferentes. Se o ficheiro não existir, fica como está.
/// Usar sempre que um caminho vem de fora (ficheiro de texto, config, utilizador).
pub fn canonical(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
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
                let c = |p: &str| canonical(Path::new(p));
                match kind {
                    "folder" => lib.folders.push(c(path)),
                    "queue" => lib.queue.push(c(path)),
                    "fav" => lib.favorites.push(c(path)),
                    "recent" => lib.recent.push(c(path)),
                    "title" => {
                        // title<TAB>caminho<TAB>título<TAB>autor (autor opcional)
                        let mut it = path.split('\t');
                        if let (Some(song), Some(title)) = (it.next(), it.next()) {
                            let composer = it.next().filter(|c| !c.is_empty()).map(str::to_owned);
                            lib.titles.push((c(song), title.to_owned(), composer));
                        }
                    }
                    "link" => {
                        if let Some((folder, song)) = path.split_once('\t') {
                            lib.links.push((c(folder), c(song)));
                        }
                    }
                    _ => {}
                }
            }
            if let Ok(mut t) = TITLES.write() {
                *t = lib.titles.clone();
            }
            // Regravar já na forma do disco (corrige ficheiros antigos)
            lib.favorites.dedup();
            lib.save();
        }

        lib.rescan();
        lib
    }

    pub fn save(&self) {
        // Os testes NUNCA escrevem a biblioteca real do utilizador
        if cfg!(test) {
            return;
        }
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
        for (song, title, composer) in &self.titles {
            out.push_str(&format!(
                "title\t{}\t{}\t{}\n",
                song.display(),
                title,
                composer.as_deref().unwrap_or("")
            ));
        }

        let path = store_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(e) = std::fs::write(&path, out) {
            log::error!("Failed to save library: {e}");
        }
    }

    /// Volta a ler as pastas **em segundo plano**: as pastas podem estar no Google
    /// Drive, que às vezes demora segundos a responder, e isso nunca pode parar a
    /// app. A lista nova entra em `songs` no próximo `poll_scan()` depois de pronta.
    pub fn rescan(&mut self) {
        let folders = self.folders.clone();
        let links = self.links.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("ler-biblioteca".into())
            .spawn(move || {
                let _ = tx.send(scan_songs(&folders, &links));
            });
        match spawned {
            Ok(_) => self.scan_rx = Some(rx),
            // Sem threads (não deve acontecer): ler já, como antes
            Err(_) => self.songs = scan_songs(&self.folders, &self.links),
        }
    }

    /// Chamar a cada frame: aplica o resultado da leitura em segundo plano.
    /// Devolve `true` se a lista de músicas mudou agora.
    pub fn poll_scan(&mut self) -> bool {
        let Some(rx) = &self.scan_rx else { return false };
        match rx.try_recv() {
            Ok(songs) => {
                self.songs = songs;
                self.scan_rx = None;
                true
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => false,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.scan_rx = None;
                false
            }
        }
    }

    /// A biblioteca ainda está a ler as pastas?
    pub fn is_scanning(&self) -> bool {
        self.scan_rx.is_some()
    }

    /// Músicas agrupadas por autor: mais músicas primeiro, depois por nome;
    /// as músicas sem autor ficam num grupo "Outros" no fim.
    pub fn by_composer(&self) -> Vec<ComposerGroup> {
        let mut groups: Vec<ComposerGroup> = Vec::new();
        let mut others = ComposerGroup {
            key: String::new(),
            name: "Outros".into(),
            songs: Vec::new(),
        };
        for (i, s) in self.songs.iter().enumerate() {
            let (Some(key), Some(name)) = (&s.composer_key, &s.composer) else {
                others.songs.push(i);
                continue;
            };
            match groups.iter_mut().find(|g| &g.key == key) {
                Some(g) => {
                    g.songs.push(i);
                    // Nome mais completo ganha ("Frédéric François Chopin" > "Chopin")
                    if name.chars().count() > g.name.chars().count() {
                        g.name = name.clone();
                    }
                }
                None => groups.push(ComposerGroup {
                    key: key.clone(),
                    name: name.clone(),
                    songs: vec![i],
                }),
            }
        }
        for g in &mut groups {
            if let Some(canon) = canonical_name(&g.key) {
                g.name = canon.into();
            }
        }
        groups.sort_by(|a, b| {
            b.songs
                .len()
                .cmp(&a.songs.len())
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        if !others.songs.is_empty() {
            groups.push(others);
        }
        groups
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
    /// Título e autor a mostrar. Ordem: título escolhido pelo utilizador
    /// (`set_title`) → nome do ficheiro. (As pastas `<Autor>/<Música>` entram via
    /// `LibSong::with_root`.)
    pub fn pretty_title(path: &Path) -> (String, Option<String>) {
        title_override(path).unwrap_or_else(|| pretty_title_of(&Self::display_name(path)))
    }

    /// Dá a uma música um título (e autor) escolhidos pelo utilizador, ou repõe o
    /// automático com `title = None`. Aplica-se já à lista de músicas.
    pub fn set_title(&mut self, song: &Path, title: Option<String>, composer: Option<String>) {
        let song = canonical(song);
        self.titles.retain(|(p, _, _)| p != &song);
        if let Some(t) = title.filter(|t| !t.trim().is_empty()) {
            self.titles.push((song.clone(), t.trim().to_owned(), composer.filter(|c| !c.trim().is_empty())));
        }
        if let Ok(mut t) = TITLES.write() {
            *t = self.titles.clone();
        }
        self.save();
        // Atualizar a música na lista sem voltar a ler as pastas
        if let Some(s) = self.songs.iter_mut().find(|s| s.path == song) {
            *s = LibSong::with_root(song.clone(), None).with_override();
        }
    }

    pub fn display_name(path: &Path) -> String {
        path.file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned())
    }
}

/// Um autor e as suas músicas (índices em `Library::songs`).
#[derive(Debug, Clone, PartialEq)]
pub struct ComposerGroup {
    /// Chave normalizada ("chopin"); vazia no grupo "Outros"
    pub key: String,
    pub name: String,
    pub songs: Vec<usize>,
}

/// Chave para agrupar nomes do mesmo autor: o apelido, sem acentos, em
/// minúsculas, com alguns nomes alternativos unificados.
/// "Frédéric François Chopin", "Frederic Chopin", "Chopin" → "chopin".
pub fn composer_key(name: &str) -> String {
    let last = name.split_whitespace().last().unwrap_or(name);
    let k: String = last
        .chars()
        .flat_map(|c| c.to_lowercase())
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            c => c,
        })
        .filter(|c| c.is_alphanumeric())
        .collect();
    match k.as_str() {
        "czajkowski" | "tchaikovski" | "tschaikowsky" | "chaikovsky" | "tchaikovsky" => {
            "tchaikovsky".into()
        }
        _ => k,
    }
}

/// Nome a mostrar para autores conhecidos, independentemente da grafia do ficheiro.
fn canonical_name(key: &str) -> Option<&'static str> {
    Some(match key {
        "chopin" => "Frédéric Chopin",
        "tchaikovsky" => "Piotr Tchaikovsky",
        "debussy" => "Claude Debussy",
        "satie" => "Erik Satie",
        "beethoven" => "Ludwig van Beethoven",
        "bach" => "Johann Sebastian Bach",
        "mozart" => "Wolfgang Amadeus Mozart",
        "pachelbel" => "Johann Pachelbel",
        "burgmuller" => "Friedrich Burgmüller",
        "petzold" => "Christian Petzold",
        "tiersen" => "Yann Tiersen",
        _ => return None,
    })
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

/// Lê as pastas e devolve as músicas (corre na thread de leitura).
fn scan_songs(folders: &[PathBuf], links: &[(PathBuf, PathBuf)]) -> Vec<LibSong> {
    let mut found: Vec<(PathBuf, PathBuf)> = Vec::new(); // (ficheiro, raiz)
    for folder in folders {
        let mut files = Vec::new();
        scan_dir(folder, 0, &mut files);
        found.extend(files.into_iter().map(|f| (f, folder.clone())));
    }
    found.retain(|(p, _)| !links.iter().any(|(folder, _)| p.starts_with(folder)));
    found.sort_by_key(|(p, _)| p.to_string_lossy().to_lowercase());
    found.dedup_by(|a, b| a.0 == b.0);
    found
        .into_iter()
        .map(|(p, root)| LibSong::with_root(p, Some(&root)))
        .collect()
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
    fn groups_by_composer_with_folders_and_filenames() {
        let root = PathBuf::from("/m");
        let mk = |p: &str| LibSong::with_root(PathBuf::from(p), Some(&root));
        let lib = Library {
            songs: vec![
                mk("/m/Frédéric François Chopin_(1810 - 1849) - Waltz in C♯ Minor.mid"),
                mk("/m/Frederic Chopin - Waltz in A Minor.mid"),
                mk("/m/Chopin/Valsa Op. 64 Nº 2/valsa.mid"),
                mk("/m/Piotr Iljicz Czajkowski - Dance of the Sugar Plum Fairy.mid"),
                mk("/m/Interstellar.mid"),
            ],
            ..Default::default()
        };
        // Convenção de pastas: título e autor das pastas
        assert_eq!(lib.songs[2].title, "Valsa Op. 64 Nº 2");
        assert_eq!(lib.songs[2].composer.as_deref(), Some("Chopin"));

        let g = lib.by_composer();
        assert_eq!(g[0].name, "Frédéric Chopin");
        assert_eq!(g[0].songs, vec![0, 1, 2]);
        assert_eq!(g[1].name, "Piotr Tchaikovsky");
        assert_eq!(g.last().unwrap().name, "Outros");
        assert_eq!(g.last().unwrap().songs, vec![4]);
        // Pesquisa pelo autor
        assert!(lib.songs[3].matches("tchaikovsky") || lib.songs[3].matches("czajkowski"));
    }

    #[test]
    fn user_titles_override_everywhere() {
        let path = PathBuf::from("/nao/existe/Frederic Chopin - Waltz in C# Minor (teste).mid");
        let mut lib = Library::default();
        lib.songs = vec![LibSong::new(path.clone())];
        lib.set_title(&path, Some("Valsa em Dó♯ menor, Op. 64 Nº 2".into()), Some("Frédéric Chopin".into()));
        assert_eq!(Library::pretty_title(&path).0, "Valsa em Dó♯ menor, Op. 64 Nº 2");
        assert_eq!(lib.songs[0].title, "Valsa em Dó♯ menor, Op. 64 Nº 2");
        assert!(lib.songs[0].matches("op. 64"));
        assert_eq!(lib.by_composer()[0].name, "Frédéric Chopin");
        lib.set_title(&path, None, None);
        assert_eq!(Library::pretty_title(&path).0, "Waltz in C# Minor (teste)");
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
