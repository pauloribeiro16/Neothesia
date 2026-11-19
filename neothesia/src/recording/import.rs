//! Importação de gravações livres (feitas fora do ecrã de tocar, ex.: no MusicaApp)
//! a partir de pastas ligadas a uma música (`Library::links`).
//!
//! Cada ficheiro `.mid` novo da pasta é comparado com a música por alinhamento
//! (`align`), copiado para o `TakeStore` e passa a aparecer nas Gravações.
//! Os ficheiros originais nunca são alterados.

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use midi_file::MidiFile;

use super::{ExpectedNote, PlayedNote, TakeStore, align::align, store::Take};
use crate::library::Library;

/// Gravações com menos notas do que isto são consideradas testes e ignoradas.
pub const MIN_IMPORT_NOTES: usize = 20;

fn notes_of(midi: &MidiFile) -> Vec<PlayedNote> {
    midi.tracks
        .iter()
        .flat_map(|t| t.notes.iter())
        .map(|n| PlayedNote {
            note: n.note,
            start: n.start,
            end: n.end,
            velocity: n.velocity,
        })
        .collect()
}

/// Importa os ficheiros novos de `folder` como gravações de `song`.
/// Devolve as gravações importadas agora.
pub fn sync_folder(store: &TakeStore, song: &Path, folder: &Path) -> Vec<Take> {
    let Ok(entries) = std::fs::read_dir(folder) else {
        log::warn!("Pasta de gravações inacessível: {folder:?}");
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| crate::scene::menu_scene::is_midi_file(p))
        .collect();
    files.sort();

    let known: HashSet<PathBuf> = store
        .list(song)
        .into_iter()
        .filter_map(|t| t.info.source)
        .collect();
    let new: Vec<PathBuf> = files.into_iter().filter(|f| !known.contains(f)).collect();
    if new.is_empty() {
        return Vec::new();
    }

    let original = match MidiFile::new(song) {
        Ok(m) => m,
        Err(e) => {
            log::error!("Não consegui abrir a música {song:?}: {e}");
            return Vec::new();
        }
    };
    let expected: Vec<ExpectedNote> = original
        .tracks
        .iter()
        .flat_map(|t| t.notes.iter())
        .map(|n| ExpectedNote { note: n.note, start: n.start })
        .collect();

    let mut imported = Vec::new();
    for src in new {
        let played = match MidiFile::new(&src) {
            Ok(m) => notes_of(&m),
            Err(e) => {
                log::warn!("Gravação ilegível {src:?}: {e}");
                continue;
            }
        };
        if played.len() < MIN_IMPORT_NOTES {
            log::info!("Ignorada (só {} notas): {src:?}", played.len());
            continue;
        }
        let a = align(&expected, &played, &original.measures);
        let recorded_at = std::fs::metadata(&src)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        match store.import_copy(song, &src, recorded_at, &played, &a) {
            Ok(take) => {
                log::info!(
                    "Importada {src:?}: {:.1}% certas, andamento {:.0}%",
                    take.accuracy * 100.0,
                    a.tempo_ratio * 100.0
                );
                imported.push(take);
            }
            Err(e) => log::error!("Falha ao importar {src:?}: {e}"),
        }
    }
    imported
}

/// Sincroniza todas as pastas ligadas da biblioteca. Devolve o nº de importadas.
pub fn sync_all(library: &Library, store: &TakeStore) -> usize {
    library
        .links
        .iter()
        .map(|(folder, song)| sync_folder(store, song, folder).len())
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recording::store::write_smf;
    use std::time::Duration;

    #[test]
    fn imports_new_files_once_and_skips_tiny_ones() {
        let base = std::env::temp_dir().join(format!("neothasia-import-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let folder = base.join("gravacoes");
        std::fs::create_dir_all(&folder).unwrap();

        // "Música" original: escala de 40 notas
        let song_notes: Vec<PlayedNote> = (0..40)
            .map(|k| PlayedNote {
                note: 60 + (k % 12) as u8,
                start: Duration::from_millis(k * 400),
                end: Duration::from_millis(k * 400 + 300),
                velocity: 80,
            })
            .collect();
        let song = base.join("musica.mid");
        write_smf(&song_notes, &song).unwrap();

        // Gravação: a mesma, mais lenta e com silêncio inicial
        let take_notes: Vec<PlayedNote> = song_notes
            .iter()
            .map(|n| PlayedNote {
                start: n.start * 3 / 2 + Duration::from_secs(3),
                end: n.end * 3 / 2 + Duration::from_secs(3),
                ..*n
            })
            .collect();
        write_smf(&take_notes, &folder.join("gravacao-1.mid")).unwrap();
        write_smf(&take_notes[..5], &folder.join("teste.mid")).unwrap();

        let store = TakeStore::with_base(base.join("store"));
        let first = sync_folder(&store, &song, &folder);
        assert_eq!(first.len(), 1, "só a gravação com notas suficientes");
        assert!(first[0].accuracy > 0.99, "{}", first[0].accuracy);
        let ratio = first[0].info.tempo_ratio.unwrap();
        assert!((ratio - 0.667).abs() < 0.02, "{ratio}");

        // Segunda sincronização: nada de novo
        assert!(sync_folder(&store, &song, &folder).is_empty());
        assert_eq!(store.list(&song).len(), 1);
    }
}
