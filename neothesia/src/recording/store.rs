//! Guarda e lista as gravações de cada música.
//!
//! `<base>/recordings/<hash da música>/<unix_secs>.mid` + `<unix_secs>.txt`
//! (linhas `chave<TAB>valor`). Ver `ARQUITETURA.md` › 7.

use std::{
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use midi_file::midly::{
    Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind,
};

use super::{PlayedNote, Report};

const TICKS_PER_BEAT: u16 = 480;
const TEMPO_MICROS_PER_BEAT: u32 = 500_000;
const TICKS_PER_SECOND: f64 = TICKS_PER_BEAT as f64 * 1_000_000.0 / TEMPO_MICROS_PER_BEAT as f64;

/// Uma gravação guardada.
#[derive(Debug, Clone, PartialEq)]
pub struct Take {
    /// Ficheiro `.mid` da gravação (abre como uma música normal)
    pub midi_path: PathBuf,
    /// Música original
    pub song: PathBuf,
    /// Segundos desde 1970 (também é o nome do ficheiro)
    pub recorded_at: u64,
    pub duration: Duration,
    pub accuracy: f32,
    pub hits: usize,
    pub missed: usize,
    pub extra: usize,
    pub offset_ms: f32,
    pub info: TakeExtra,
}

/// Dados adicionais (sobretudo de gravações importadas, comparadas por alinhamento).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TakeExtra {
    /// Ficheiro de onde foi importada (para não importar duas vezes)
    pub source: Option<PathBuf>,
    /// Velocidade do utilizador / velocidade original (ex.: 0.64 = 64 %)
    pub tempo_ratio: Option<f32>,
    /// Regularidade do ritmo em ms (menor = melhor)
    pub spread_ms: f32,
    /// Pausas para pensar/corrigir
    pub hesitations: usize,
    /// Trecho da música coberto (tempo do original)
    pub covered: Option<(Duration, Duration)>,
}

impl Take {
    /// "20/11/2026 00:31" (hora UTC; suficiente para distinguir gravações)
    pub fn date_label(&self) -> String {
        let secs = self.recorded_at as i64;
        let days = secs.div_euclid(86_400);
        let rem = secs.rem_euclid(86_400);
        let (y, m, d) = civil_from_days(days);
        format!("{d:02}/{m:02}/{y} {:02}:{:02}", rem / 3600, (rem % 3600) / 60)
    }
}

/// Converte dias desde 1970-01-01 em (ano, mês, dia) — algoritmo de H. Hinnant.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

pub struct TakeStore {
    base: PathBuf,
}

impl Default for TakeStore {
    fn default() -> Self {
        Self::new()
    }
}

impl TakeStore {
    /// Pasta por omissão: `~/.config/neothesia/recordings`
    pub fn new() -> Self {
        let base = std::env::var_os("HOME")
            .filter(|h| !h.is_empty())
            .map(|h| PathBuf::from(h).join(".config").join("neothesia"))
            .unwrap_or_else(|| PathBuf::from("."))
            .join("recordings");
        Self { base }
    }

    pub fn with_base(base: PathBuf) -> Self {
        Self { base }
    }

    /// O caminho é uma gravação (está dentro da pasta de gravações)?
    pub fn is_take_path(&self, path: &Path) -> bool {
        path.starts_with(&self.base)
    }

    fn song_dir(&self, song: &Path) -> PathBuf {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        song.hash(&mut h);
        self.base.join(format!("{:016x}", h.finish()))
    }

    /// Grava uma nova gravação. Devolve `None` se não havia notas.
    pub fn save(&self, song: &Path, notes: &[PlayedNote], report: &Report) -> anyhow::Result<Option<Take>> {
        if notes.is_empty() {
            return Ok(None);
        }
        let dir = self.song_dir(song);
        std::fs::create_dir_all(&dir)?;

        let mut recorded_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        // Evitar colisões se duas gravações acabarem no mesmo segundo
        while dir.join(format!("{recorded_at}.mid")).exists() {
            recorded_at += 1;
        }

        let duration = notes.iter().map(|n| n.end).max().unwrap_or_default();
        let midi_path = dir.join(format!("{recorded_at}.mid"));
        write_smf(notes, &midi_path)?;

        let take = Take {
            midi_path,
            song: song.to_path_buf(),
            recorded_at,
            duration,
            accuracy: report.accuracy,
            hits: report.hits,
            missed: report.missed,
            extra: report.extra,
            offset_ms: report.mean_offset_ms,
            info: TakeExtra {
                spread_ms: report.offset_spread_ms,
                hesitations: report.hesitations,
                ..Default::default()
            },
        };
        std::fs::write(dir.join(format!("{recorded_at}.txt")), meta_to_text(&take))?;
        Ok(Some(take))
    }

    /// Importa uma gravação externa: copia o ficheiro tal como está e guarda os
    /// resultados do alinhamento. Ver `recording::import`.
    pub fn import_copy(
        &self,
        song: &Path,
        src: &Path,
        recorded_at: u64,
        played: &[PlayedNote],
        a: &super::align::Alignment,
    ) -> anyhow::Result<Take> {
        let dir = self.song_dir(song);
        std::fs::create_dir_all(&dir)?;
        let mut recorded_at = recorded_at;
        while dir.join(format!("{recorded_at}.mid")).exists() {
            recorded_at += 1;
        }
        let midi_path = dir.join(format!("{recorded_at}.mid"));
        std::fs::copy(src, &midi_path)?;

        let first = played.iter().map(|n| n.start).min().unwrap_or_default();
        let last = played.iter().map(|n| n.end).max().unwrap_or_default();
        let r = &a.report;
        let take = Take {
            midi_path,
            song: song.to_path_buf(),
            recorded_at,
            duration: last.saturating_sub(first),
            accuracy: r.accuracy,
            hits: r.hits,
            missed: r.missed,
            extra: r.extra,
            offset_ms: 0.0,
            info: TakeExtra {
                source: Some(src.to_path_buf()),
                tempo_ratio: Some(a.tempo_ratio),
                spread_ms: r.offset_spread_ms,
                hesitations: r.hesitations,
                covered: Some((a.covered_start, a.covered_end)),
            },
        };
        std::fs::write(dir.join(format!("{recorded_at}.txt")), meta_to_text(&take))?;
        Ok(take)
    }

    /// Gravações de uma música, da mais recente para a mais antiga.
    pub fn list(&self, song: &Path) -> Vec<Take> {
        let dir = self.song_dir(song);
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return Vec::new();
        };
        let mut takes: Vec<Take> = entries
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == "txt"))
            .filter_map(|e| {
                let text = std::fs::read_to_string(e.path()).ok()?;
                let stem = e.path().file_stem()?.to_str()?.parse::<u64>().ok()?;
                let midi_path = dir.join(format!("{stem}.mid"));
                midi_path.exists().then(|| meta_from_text(&text, midi_path, stem))?
            })
            .filter(|t| t.song == song)
            .collect();
        takes.sort_by(|a, b| b.recorded_at.cmp(&a.recorded_at));
        takes
    }

    /// Todas as gravações de todas as músicas (para estatísticas).
    pub fn list_all(&self) -> Vec<Take> {
        let Ok(dirs) = std::fs::read_dir(&self.base) else {
            return Vec::new();
        };
        let mut all = Vec::new();
        for d in dirs.flatten() {
            let Ok(files) = std::fs::read_dir(d.path()) else { continue };
            for f in files.flatten() {
                let p = f.path();
                if p.extension().is_none_or(|x| x != "txt") {
                    continue;
                }
                let (Ok(text), Some(stem)) = (
                    std::fs::read_to_string(&p),
                    p.file_stem().and_then(|s| s.to_str()).and_then(|s| s.parse::<u64>().ok()),
                ) else {
                    continue;
                };
                let midi_path = d.path().join(format!("{stem}.mid"));
                if midi_path.exists()
                    && let Some(t) = meta_from_text(&text, midi_path, stem)
                {
                    all.push(t);
                }
            }
        }
        all.sort_by(|a, b| b.recorded_at.cmp(&a.recorded_at));
        all
    }

    pub fn delete(&self, take: &Take) -> std::io::Result<()> {
        std::fs::remove_file(&take.midi_path)?;
        let _ = std::fs::remove_file(take.midi_path.with_extension("txt"));
        Ok(())
    }
}

fn meta_to_text(t: &Take) -> String {
    let mut s = format!(
        "song\t{}\nduration_ms\t{}\naccuracy\t{}\nhits\t{}\nmissed\t{}\nextra\t{}\noffset_ms\t{}\nspread_ms\t{}\nhesitations\t{}\n",
        t.song.display(),
        t.duration.as_millis(),
        t.accuracy,
        t.hits,
        t.missed,
        t.extra,
        t.offset_ms,
        t.info.spread_ms,
        t.info.hesitations,
    );
    if let Some(src) = &t.info.source {
        s.push_str(&format!("source\t{}\n", src.display()));
    }
    if let Some(r) = t.info.tempo_ratio {
        s.push_str(&format!("tempo_ratio\t{r}\n"));
    }
    if let Some((a, b)) = t.info.covered {
        s.push_str(&format!("covered_ms\t{}-{}\n", a.as_millis(), b.as_millis()));
    }
    s
}

fn meta_from_text(text: &str, midi_path: PathBuf, recorded_at: u64) -> Option<Take> {
    let mut take = Take {
        midi_path,
        song: PathBuf::new(),
        recorded_at,
        duration: Duration::ZERO,
        accuracy: 0.0,
        hits: 0,
        missed: 0,
        extra: 0,
        offset_ms: 0.0,
        info: TakeExtra::default(),
    };
    for line in text.lines() {
        let Some((k, v)) = line.split_once('\t') else { continue };
        match k {
            "song" => take.song = PathBuf::from(v),
            "duration_ms" => take.duration = Duration::from_millis(v.parse().unwrap_or(0)),
            "accuracy" => take.accuracy = v.parse().unwrap_or(0.0),
            "hits" => take.hits = v.parse().unwrap_or(0),
            "missed" => take.missed = v.parse().unwrap_or(0),
            "extra" => take.extra = v.parse().unwrap_or(0),
            "offset_ms" => take.offset_ms = v.parse().unwrap_or(0.0),
            "source" => take.info.source = Some(PathBuf::from(v)),
            "tempo_ratio" => take.info.tempo_ratio = v.parse().ok(),
            "spread_ms" => take.info.spread_ms = v.parse().unwrap_or(0.0),
            "hesitations" => take.info.hesitations = v.parse().unwrap_or(0),
            "covered_ms" => {
                if let Some((a, b)) = v.split_once('-')
                    && let (Ok(a), Ok(b)) = (a.parse(), b.parse())
                {
                    take.info.covered = Some((Duration::from_millis(a), Duration::from_millis(b)));
                }
            }
            _ => {}
        }
    }
    (!take.song.as_os_str().is_empty()).then_some(take)
}

fn to_ticks(d: Duration) -> u64 {
    (d.as_secs_f64() * TICKS_PER_SECOND).round() as u64
}

/// Escreve as notas como SMF formato 0 (120 bpm, 480 ticks/semínima).
pub fn write_smf(notes: &[PlayedNote], path: &Path) -> anyhow::Result<()> {
    // (tick, ordem, mensagem): NoteOff antes de NoteOn no mesmo tick
    let mut events: Vec<(u64, u8, MidiMessage)> = Vec::with_capacity(notes.len() * 2);
    for n in notes {
        events.push((
            to_ticks(n.start),
            1,
            MidiMessage::NoteOn { key: n.note.into(), vel: n.velocity.max(1).into() },
        ));
        events.push((
            to_ticks(n.end).max(to_ticks(n.start) + 1),
            0,
            MidiMessage::NoteOff { key: n.note.into(), vel: 0.into() },
        ));
    }
    events.sort_by_key(|(t, o, _)| (*t, *o));

    let mut track: Vec<TrackEvent<'static>> = vec![TrackEvent {
        delta: 0.into(),
        kind: TrackEventKind::Meta(MetaMessage::Tempo(TEMPO_MICROS_PER_BEAT.into())),
    }];
    let mut last = 0u64;
    for (tick, _, message) in events {
        let delta = (tick - last).min(u32::MAX as u64 >> 4) as u32;
        last = tick;
        track.push(TrackEvent {
            delta: delta.into(),
            kind: TrackEventKind::Midi { channel: 0.into(), message },
        });
    }
    track.push(TrackEvent {
        delta: 0.into(),
        kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
    });

    let smf = Smf {
        header: Header::new(Format::SingleTrack, Timing::Metrical(TICKS_PER_BEAT.into())),
        tracks: vec![track],
    };
    smf.save(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("neothasia-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    fn notes() -> Vec<PlayedNote> {
        vec![
            PlayedNote { note: 60, start: Duration::from_millis(0), end: Duration::from_millis(400), velocity: 90 },
            PlayedNote { note: 64, start: Duration::from_millis(500), end: Duration::from_millis(900), velocity: 70 },
        ]
    }

    #[test]
    fn save_list_load_delete_roundtrip() {
        let store = TakeStore::with_base(tmp_dir("store"));
        let song = PathBuf::from("/musicas/Für Elise.mid");
        let report = Report { accuracy: 0.75, hits: 3, missed: 1, ..Default::default() };

        let take = store.save(&song, &notes(), &report).unwrap().unwrap();
        assert_eq!(take.duration, Duration::from_millis(900));

        let list = store.list(&song);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].accuracy, 0.75);
        assert_eq!(list[0].song, song);
        assert_eq!(store.list_all().len(), 1);

        // O .mid gravado abre com o leitor da app e tem as mesmas notas
        let midi = midi_file::MidiFile::new(&take.midi_path).unwrap();
        let n: Vec<u8> = midi.tracks.iter().flat_map(|t| t.notes.iter().map(|n| n.note)).collect();
        assert_eq!(n, vec![60, 64]);
        let first = &midi.tracks.iter().find(|t| !t.notes.is_empty()).unwrap().notes[0];
        assert!((first.end.as_millis() as i64 - 400).abs() <= 2);

        store.delete(&take).unwrap();
        assert!(store.list(&song).is_empty());
    }

    #[test]
    fn empty_take_is_not_saved() {
        let store = TakeStore::with_base(tmp_dir("empty"));
        let r = store.save(Path::new("/x.mid"), &[], &Report::default()).unwrap();
        assert!(r.is_none());
    }

    #[test]
    fn date_label_formats_utc() {
        let t = Take {
            midi_path: PathBuf::new(),
            song: PathBuf::new(),
            recorded_at: 1_795_000_000,
            duration: Duration::ZERO,
            accuracy: 0.0,
            hits: 0,
            missed: 0,
            extra: 0,
            offset_ms: 0.0,
            info: TakeExtra::default(),
        };
        let label = t.date_label();
        assert_eq!(label, "18/11/2026 11:06");
    }
}
