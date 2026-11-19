//! Gravação automática do que o utilizador toca no ecrã de tocar.
//! Ver `ARQUITETURA.md` › 7.

use std::{path::PathBuf, time::Duration};

use midi_file::midly::MidiMessage;

use crate::{
    context::Context,
    recording::{
        ExpectedNote, SessionResult, TakeRecorder, compare, compare::DEFAULT_TOLERANCE,
    },
    song::{PlayerConfig, Song},
};

/// Mínimo de notas tocadas para guardar a gravação e mostrar o resultado
/// (evita resultados por um toque acidental).
const MIN_NOTES: usize = 3;

/// Um salto para trás maior do que isto (rewind / loop) recomeça a gravação.
const REWIND_THRESHOLD: Duration = Duration::from_millis(500);

pub struct TakeSession {
    recorder: TakeRecorder,
    /// Música original. `None` = não gravar (ficheiro desconhecido ou é uma gravação).
    song_path: Option<PathBuf>,
    last_t: Duration,
    finished: bool,
}

impl TakeSession {
    pub fn new(ctx: &Context) -> Self {
        let song_path = ctx
            .config
            .last_opened_song()
            .cloned()
            .filter(|p| !ctx.takes.is_take_path(p));
        Self {
            recorder: TakeRecorder::new(),
            song_path,
            last_t: Duration::ZERO,
            finished: false,
        }
    }

    /// Chamado a cada frame com o tempo da música (sem lead-in).
    pub fn tick(&mut self, t: Duration) {
        if t + REWIND_THRESHOLD < self.last_t {
            log::info!("Rewind detetado: a gravação recomeça");
            self.recorder = TakeRecorder::new();
        }
        self.last_t = t;
    }

    pub fn midi_event(&mut self, message: &MidiMessage, t: Duration) {
        if self.song_path.is_some() && !self.finished {
            self.recorder.midi_event(message, t);
        }
    }

    /// Fecha a gravação: compara, guarda e deixa o resultado em `ctx.last_result`.
    /// Seguro de chamar mais do que uma vez (só a primeira conta).
    pub fn finish(&mut self, ctx: &mut Context, song: &Song, t: Duration) {
        if self.finished {
            return;
        }
        self.finished = true;

        let Some(song_path) = self.song_path.clone() else {
            return;
        };
        if self.recorder.note_count() < MIN_NOTES {
            return;
        }

        let played = std::mem::take(&mut self.recorder).finish(t);
        let end = t + DEFAULT_TOLERANCE;
        let expected = expected_notes(song, end);
        let report = compare(&expected, &played, &song.file.measures, DEFAULT_TOLERANCE);

        match ctx.takes.save(&song_path, &played, &report) {
            Ok(Some(take)) => {
                log::info!(
                    "Gravação guardada: {:?} ({:.0}%)",
                    take.midi_path,
                    report.accuracy * 100.0
                );
                ctx.last_result = Some(SessionResult { take, report });
            }
            Ok(None) => {}
            Err(e) => log::error!("Falha ao guardar a gravação: {e}"),
        }
    }
}

/// Notas que o utilizador devia tocar até `until`: as das faixas em modo Humano;
/// se não houver nenhuma, as de todas as faixas visíveis.
fn expected_notes(song: &Song, until: Duration) -> Vec<ExpectedNote> {
    let tracks = &song.config.tracks;
    let human: Vec<usize> = tracks
        .iter()
        .filter(|t| t.player == PlayerConfig::Human)
        .map(|t| t.track_id)
        .collect();
    let chosen: Vec<usize> = if human.is_empty() {
        tracks.iter().filter(|t| t.visible).map(|t| t.track_id).collect()
    } else {
        human
    };

    song.file
        .tracks
        .iter()
        .filter(|t| chosen.contains(&t.track_id))
        .flat_map(|t| t.notes.iter())
        .filter(|n| n.start <= until)
        .map(|n| ExpectedNote {
            note: n.note,
            start: n.start,
        })
        .collect()
}
