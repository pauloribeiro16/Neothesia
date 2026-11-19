//! Dados de demonstração para o modo de captura (`NEOTHESIA_PAGE=results|recordings`),
//! para se poder desenhar as páginas sem tocar uma música. Nunca escreve na pasta
//! real de gravações: usa uma pasta temporária.

use std::{path::PathBuf, time::Duration};

use super::{PlayedNote, Report, SectionStats, SessionResult, Take, TakeStore};
use crate::context::Context;

pub fn song_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("test.mid")
}

/// Pseudo-aleatório determinístico (para as capturas serem sempre iguais)
fn noise(i: usize) -> usize {
    (i.wrapping_mul(2654435761) >> 7) % 7
}

pub fn report(accuracy: f32) -> Report {
    let sections: Vec<SectionStats> = (0..32)
        .map(|i| {
            let errs = noise(i).saturating_sub(3) + usize::from(i % 11 == 7) * 4;
            SectionStats {
                start: Duration::from_secs(i as u64 * 2),
                hits: 8,
                missed: errs / 2,
                extra: errs - errs / 2,
            }
        })
        .collect();
    let expected = 300;
    let hits = (expected as f32 * accuracy) as usize;
    Report {
        expected,
        hits,
        missed: expected - hits,
        extra: 9,
        accuracy,
        mean_offset_ms: 23.0,
        offset_spread_ms: 41.0,
        hesitations: 3,
        sections,
    }
}

pub fn session_result() -> SessionResult {
    let report = report(0.87);
    SessionResult {
        take: Take {
            midi_path: PathBuf::from("/tmp/demo.mid"),
            song: song_path(),
            recorded_at: 1_795_000_000,
            duration: Duration::from_secs(64),
            accuracy: report.accuracy,
            hits: report.hits,
            missed: report.missed,
            extra: report.extra,
            offset_ms: report.mean_offset_ms,
            info: Default::default(),
        },
        report,
    }
}

/// Troca `ctx.takes` por uma pasta temporária com 8 gravações a melhorar.
pub fn seed_takes(ctx: &mut Context) {
    let base = std::env::temp_dir().join("neothasia-demo-recordings");
    let _ = std::fs::remove_dir_all(&base);
    ctx.takes = TakeStore::with_base(base);

    let notes = [PlayedNote {
        note: 60,
        start: Duration::ZERO,
        end: Duration::from_millis(500),
        velocity: 80,
    }];
    for (i, acc) in [0.52, 0.58, 0.55, 0.66, 0.71, 0.69, 0.80, 0.87].iter().enumerate() {
        let _ = ctx.takes.save(&song_path(), &notes, &report(*acc));
        let _ = i;
    }
}
