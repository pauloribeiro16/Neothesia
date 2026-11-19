//! Compara as notas tocadas com as esperadas.
//!
//! Emparelhamento: para cada nota esperada (por ordem de início), escolhe a nota
//! tocada **da mesma altura** mais próxima no tempo, dentro de ±`tolerance`, que
//! ainda não tenha sido usada.

use std::time::Duration;

use super::PlayedNote;

pub const DEFAULT_TOLERANCE: Duration = Duration::from_millis(150);

/// Nota que era suposto tocar (vem do ficheiro MIDI).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpectedNote {
    pub note: u8,
    pub start: Duration,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SectionStats {
    pub start: Duration,
    pub hits: usize,
    pub missed: usize,
    pub extra: usize,
}

impl SectionStats {
    /// Erros no compasso (falhadas + a mais)
    pub fn errors(&self) -> usize {
        self.missed + self.extra
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Report {
    pub expected: usize,
    pub hits: usize,
    pub missed: usize,
    pub extra: usize,
    /// hits / expected (0..1). 0 se não havia notas esperadas.
    pub accuracy: f32,
    /// Média do desvio nas notas certas: > 0 atrasado, < 0 adiantado
    pub mean_offset_ms: f32,
    /// Regularidade do ritmo em ms (menor = mais regular)
    pub offset_spread_ms: f32,
    /// Pausas para pensar/corrigir (só calculado no alinhamento livre)
    pub hesitations: usize,
    /// Estatísticas por compasso (vazio se não foram dados compassos)
    pub sections: Vec<SectionStats>,
}

impl Report {
    /// Texto curto para o ritmo, ex.: "ligeiramente atrasado".
    pub fn timing_label(&self) -> &'static str {
        let m = self.mean_offset_ms;
        if self.hits == 0 {
            "—"
        } else if m.abs() < 15.0 {
            "certinho"
        } else if m.abs() < 40.0 {
            if m > 0.0 { "ligeiramente atrasado" } else { "ligeiramente adiantado" }
        } else if m > 0.0 {
            "atrasado"
        } else {
            "adiantado"
        }
    }
}

/// `bars`: inícios dos compassos (ex.: `MidiFile::measures`), ordenados.
pub fn compare(
    expected: &[ExpectedNote],
    played: &[PlayedNote],
    bars: &[Duration],
    tolerance: Duration,
) -> Report {
    let mut expected: Vec<ExpectedNote> = expected.to_vec();
    expected.sort_by_key(|n| (n.start, n.note));
    let mut played: Vec<PlayedNote> = played.to_vec();
    played.sort_by_key(|n| (n.start, n.note));

    let mut used = vec![false; played.len()];
    let mut offsets: Vec<f32> = Vec::new();

    let mut sections: Vec<SectionStats> = bars
        .iter()
        .map(|&start| SectionStats {
            start,
            ..Default::default()
        })
        .collect();
    let section_of = |t: Duration| -> Option<usize> {
        if bars.is_empty() {
            None
        } else {
            Some(bars.partition_point(|&b| b <= t).saturating_sub(1))
        }
    };

    let mut hits = 0;
    let mut missed = 0;

    for e in &expected {
        let lo = e.start.saturating_sub(tolerance);
        let hi = e.start + tolerance;
        let from = played.partition_point(|p| p.start < lo);

        let mut best: Option<(usize, i128)> = None;
        for (i, p) in played.iter().enumerate().skip(from) {
            if p.start > hi {
                break;
            }
            if used[i] || p.note != e.note {
                continue;
            }
            let diff = p.start.as_micros() as i128 - e.start.as_micros() as i128;
            if best.is_none_or(|(_, d)| diff.abs() < d.abs()) {
                best = Some((i, diff));
            }
        }

        let sec = section_of(e.start);
        match best {
            Some((i, diff)) => {
                used[i] = true;
                hits += 1;
                offsets.push(diff as f32 / 1000.0);
                if let Some(s) = sec {
                    sections[s].hits += 1;
                }
            }
            None => {
                missed += 1;
                if let Some(s) = sec {
                    sections[s].missed += 1;
                }
            }
        }
    }

    let mut extra = 0;
    for (i, p) in played.iter().enumerate() {
        if !used[i] {
            extra += 1;
            if let Some(s) = section_of(p.start) {
                sections[s].extra += 1;
            }
        }
    }

    let mean = if offsets.is_empty() {
        0.0
    } else {
        offsets.iter().sum::<f32>() / offsets.len() as f32
    };
    let spread = if offsets.len() < 2 {
        0.0
    } else {
        (offsets.iter().map(|o| (o - mean).powi(2)).sum::<f32>() / offsets.len() as f32).sqrt()
    };

    Report {
        expected: expected.len(),
        hits,
        missed,
        extra,
        accuracy: if expected.is_empty() {
            0.0
        } else {
            hits as f32 / expected.len() as f32
        },
        mean_offset_ms: mean,
        offset_spread_ms: spread,
        hesitations: 0,
        sections,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(v: u64) -> Duration {
        Duration::from_millis(v)
    }
    fn exp(note: u8, t: u64) -> ExpectedNote {
        ExpectedNote { note, start: ms(t) }
    }
    fn play(note: u8, t: u64) -> PlayedNote {
        PlayedNote { note, start: ms(t), end: ms(t + 100), velocity: 80 }
    }

    #[test]
    fn perfect_take() {
        let e = [exp(60, 0), exp(62, 500), exp(64, 1000)];
        let p = [play(60, 0), play(62, 500), play(64, 1000)];
        let r = compare(&e, &p, &[], DEFAULT_TOLERANCE);
        assert_eq!((r.hits, r.missed, r.extra), (3, 0, 0));
        assert_eq!(r.accuracy, 1.0);
        assert_eq!(r.mean_offset_ms, 0.0);
    }

    #[test]
    fn wrong_note_counts_as_missed_and_extra() {
        let e = [exp(60, 0)];
        let p = [play(61, 0)];
        let r = compare(&e, &p, &[], DEFAULT_TOLERANCE);
        assert_eq!((r.hits, r.missed, r.extra), (0, 1, 1));
    }

    #[test]
    fn outside_tolerance_is_missed() {
        let e = [exp(60, 0)];
        let p = [play(60, 400)];
        let r = compare(&e, &p, &[], DEFAULT_TOLERANCE);
        assert_eq!((r.hits, r.missed, r.extra), (0, 1, 1));
    }

    #[test]
    fn late_playing_gives_positive_offset() {
        let e = [exp(60, 0), exp(60, 1000)];
        let p = [play(60, 50), play(60, 1050)];
        let r = compare(&e, &p, &[], DEFAULT_TOLERANCE);
        assert_eq!(r.hits, 2);
        assert!((r.mean_offset_ms - 50.0).abs() < 0.01);
        assert_eq!(r.timing_label(), "atrasado");
    }

    #[test]
    fn each_played_note_used_once_for_chords_and_repeats() {
        // duas notas iguais muito próximas, só uma tocada
        let e = [exp(60, 0), exp(60, 100)];
        let p = [play(60, 90)];
        let r = compare(&e, &p, &[], DEFAULT_TOLERANCE);
        assert_eq!((r.hits, r.missed, r.extra), (1, 1, 0));
    }

    #[test]
    fn sections_by_bar() {
        let bars = [ms(0), ms(2000)];
        let e = [exp(60, 0), exp(62, 2100)];
        let p = [play(60, 10), play(70, 2200)];
        let r = compare(&e, &p, &bars, DEFAULT_TOLERANCE);
        assert_eq!(r.sections.len(), 2);
        assert_eq!(r.sections[0].hits, 1);
        assert_eq!(r.sections[1].missed, 1);
        assert_eq!(r.sections[1].extra, 1);
        assert_eq!(r.sections[1].errors(), 2);
    }
}
