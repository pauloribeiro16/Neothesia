//! Comparação por **alinhamento de sequência**, independente do andamento.
//!
//! Serve para gravações livres (feitas fora do ecrã de tocar, ex.: no MusicaApp):
//! o utilizador pode tocar mais devagar, começar a gravar com silêncio, ou tocar só
//! um trecho. Em vez de comparar tempos absolutos, alinha-se a sequência de
//! acordes (notas que começam quase juntas) com programação dinâmica:
//!
//! - **semi-global**: o trecho tocado pode começar e acabar em qualquer ponto da
//!   música, sem penalização pelo que fica de fora;
//! - **por acordes**: a ordem das notas dentro de um acorde não importa;
//! - depois do alinhamento, uma regressão linear (tempo tocado vs. tempo original)
//!   dá o **andamento relativo** e a **regularidade** (resíduos em ms).

use std::time::Duration;

use super::{ExpectedNote, PlayedNote, Report, SectionStats};

/// Notas que começam a menos disto contam como o mesmo acorde.
const CHORD_WINDOW: Duration = Duration::from_millis(60);

#[derive(Debug, Clone)]
struct Chord {
    t: f64,
    notes: Vec<u8>,
}

fn group<I: IntoIterator<Item = (Duration, u8)>>(items: I) -> Vec<Chord> {
    let mut v: Vec<(Duration, u8)> = items.into_iter().collect();
    v.sort();
    let mut out: Vec<Chord> = Vec::new();
    let mut anchor = Duration::ZERO;
    for (t, n) in v {
        match out.last_mut() {
            Some(c) if t.saturating_sub(anchor) <= CHORD_WINDOW => {
                if !c.notes.contains(&n) {
                    c.notes.push(n);
                }
            }
            _ => {
                anchor = t;
                out.push(Chord { t: t.as_secs_f64(), notes: vec![n] });
            }
        }
    }
    for c in &mut out {
        c.notes.sort_unstable();
    }
    out
}

fn common(a: &[u8], b: &[u8]) -> usize {
    // ambos ordenados
    let (mut i, mut j, mut n) = (0, 0, 0);
    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            std::cmp::Ordering::Equal => {
                n += 1;
                i += 1;
                j += 1;
            }
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
        }
    }
    n
}

// Pontuações do alinhamento
const HIT: f32 = 2.0;
const WRONG: f32 = -1.0; // por nota não partilhada num par emparelhado
const SKIP: f32 = -1.0; // por nota de um acorde saltado (falhado ou a mais)

#[derive(Clone, Copy, PartialEq, Eq)]
enum Move {
    None,
    Diag,
    Up,   // salta acorde esperado (falhado)
    Left, // salta acorde tocado (a mais)
}

/// Resultado do alinhamento: o `Report` e o andamento relativo
/// (`tempo_ratio` = velocidade do utilizador / velocidade original).
#[derive(Debug, Clone, PartialEq)]
pub struct Alignment {
    pub report: Report,
    pub tempo_ratio: f32,
    /// Trecho da música coberto pela gravação (tempo do original)
    pub covered_start: Duration,
    pub covered_end: Duration,
}

/// Regularidade local: para cada acorde emparelhado, ajusta uma reta só aos
/// vizinhos (±`LOCAL_WINDOW` acordes) e mede o desvio desse acorde. Ignora o
/// rubato e os abrandamentos graduais.
///
/// Devolve (mediana do desvio absoluto em ms, nº de hesitações), em que uma
/// hesitação é um acorde que chega mais de `HESITATION` atrasado em relação aos
/// vizinhos (uma pausa para pensar ou corrigir).
const LOCAL_WINDOW: usize = 8;
const HESITATION: f64 = 0.35;

fn local_timing(xs: &[f64], ys: &[f64]) -> (f64, usize) {
    let n = xs.len();
    if n < 4 {
        return (0.0, 0);
    }
    let mut abs_res: Vec<f64> = Vec::with_capacity(n);
    let mut hesitations = 0;
    for i in 0..n {
        let lo = i.saturating_sub(LOCAL_WINDOW);
        let hi = (i + LOCAL_WINDOW + 1).min(n);
        let (wx, wy) = (&xs[lo..hi], &ys[lo..hi]);
        let k = wx.len() as f64;
        let (mx, my) = (wx.iter().sum::<f64>() / k, wy.iter().sum::<f64>() / k);
        let sxx: f64 = wx.iter().map(|x| (x - mx).powi(2)).sum();
        let sxy: f64 = wx.iter().zip(wy).map(|(x, y)| (x - mx) * (y - my)).sum();
        let a = if sxx > 1e-9 { sxy / sxx } else { 1.0 };
        // Desvio do intervalo para o acorde anterior (mais robusto a saltos)
        let r = if i == 0 {
            0.0
        } else {
            (ys[i] - ys[i - 1]) - a * (xs[i] - xs[i - 1])
        };
        if r > HESITATION {
            hesitations += 1;
        }
        abs_res.push(r.abs());
    }
    abs_res.sort_by(|a, b| a.partial_cmp(b).unwrap());
    (abs_res[n / 2] * 1000.0, hesitations)
}

pub fn align(expected: &[ExpectedNote], played: &[PlayedNote], bars: &[Duration]) -> Alignment {
    let e = group(expected.iter().map(|n| (n.start, n.note)));
    let p = group(played.iter().map(|n| (n.start, n.note)));
    let (m, n) = (e.len(), p.len());

    if m == 0 || n == 0 {
        return Alignment {
            report: Report {
                expected: 0,
                extra: played.len(),
                ..Default::default()
            },
            tempo_ratio: 1.0,
            covered_start: Duration::ZERO,
            covered_end: Duration::ZERO,
        };
    }

    // dp por linhas (i = esperado, j = tocado) + matriz de movimentos
    let mut prev = vec![0.0f32; n + 1];
    let mut cur = vec![0.0f32; n + 1];
    let mut moves = vec![Move::None; (m + 1) * (n + 1)];
    let idx = |i: usize, j: usize| i * (n + 1) + j;

    // Linha 0: tocar acordes antes de qualquer esperado = a mais
    for j in 1..=n {
        prev[j] = prev[j - 1] + SKIP * p[j - 1].notes.len() as f32;
        moves[idx(0, j)] = Move::Left;
    }

    let mut best = (f32::MIN, 0usize); // melhor célula na última coluna (fim livre)
    for i in 1..=m {
        // Coluna 0: saltar esperados no início é grátis (começo livre)
        cur[0] = 0.0;
        moves[idx(i, 0)] = Move::None;
        let ei = &e[i - 1].notes;
        for j in 1..=n {
            let pj = &p[j - 1].notes;
            let c = common(ei, pj);
            let diag = prev[j - 1] + HIT * c as f32
                + WRONG * ((ei.len() - c) + (pj.len() - c)) as f32;
            let up = prev[j] + SKIP * ei.len() as f32;
            let left = cur[j - 1] + SKIP * pj.len() as f32;
            let (v, mv) = if diag >= up && diag >= left && c > 0 {
                (diag, Move::Diag)
            } else if up >= left {
                (up, Move::Up)
            } else {
                (left, Move::Left)
            };
            cur[j] = v;
            moves[idx(i, j)] = mv;
        }
        if cur[n] > best.0 {
            best = (cur[n], i);
        }
        std::mem::swap(&mut prev, &mut cur);
    }

    // Traceback a partir de (best_i, n) — o fim da música depois disso é grátis
    let mut pairs: Vec<(usize, usize, usize)> = Vec::new(); // (ei, pj, comuns)
    let mut missed_chords: Vec<usize> = Vec::new();
    let mut extra_chords: Vec<usize> = Vec::new();
    let (mut i, mut j) = (best.1, n);
    while i > 0 || j > 0 {
        match moves[idx(i, j)] {
            Move::Diag => {
                pairs.push((i - 1, j - 1, common(&e[i - 1].notes, &p[j - 1].notes)));
                i -= 1;
                j -= 1;
            }
            Move::Up => {
                missed_chords.push(i - 1);
                i -= 1;
            }
            Move::Left => {
                extra_chords.push(j - 1);
                j -= 1;
            }
            Move::None => break, // início livre
        }
    }
    pairs.reverse();

    let Some(&(first_e, _, _)) = pairs.first() else {
        return Alignment {
            report: Report { extra: played.len(), ..Default::default() },
            tempo_ratio: 1.0,
            covered_start: Duration::ZERO,
            covered_end: Duration::ZERO,
        };
    };
    let last_e = pairs.last().unwrap().0;

    // Contagens por nota, só no trecho coberto [first_e, last_e]
    let covered = first_e..=last_e;
    let expected_notes: usize = e[covered.clone()].iter().map(|c| c.notes.len()).sum();
    let hits: usize = pairs.iter().map(|p| p.2).sum();
    let missed = expected_notes - hits.min(expected_notes);
    let extra = played.len().saturating_sub(hits);

    // Andamento e regularidade: regressão tocado = a·original + b
    let xs: Vec<f64> = pairs.iter().map(|&(ei, _, _)| e[ei].t).collect();
    let ys: Vec<f64> = pairs.iter().map(|&(_, pj, _)| p[pj].t).collect();
    let k = xs.len() as f64;
    let (mx, my) = (xs.iter().sum::<f64>() / k, ys.iter().sum::<f64>() / k);
    let sxx: f64 = xs.iter().map(|x| (x - mx).powi(2)).sum();
    let sxy: f64 = xs.iter().zip(&ys).map(|(x, y)| (x - mx) * (y - my)).sum();
    let a = if sxx > 1e-9 { sxy / sxx } else { 1.0 };
    let b = my - a * mx;
    let (spread_ms, hesitations) = local_timing(&xs, &ys);

    // Secções por compasso (tempo do original), só dentro do trecho coberto
    let mut sections: Vec<SectionStats> = bars
        .iter()
        .map(|&start| SectionStats { start, ..Default::default() })
        .collect();
    let sec = |t: f64| -> Option<usize> {
        if bars.is_empty() {
            return None;
        }
        let d = Duration::from_secs_f64(t.max(0.0));
        Some(bars.partition_point(|&b| b <= d).saturating_sub(1))
    };
    for &(ei, pj, c) in &pairs {
        if let Some(s) = sec(e[ei].t) {
            sections[s].hits += c;
            sections[s].missed += e[ei].notes.len() - c;
            sections[s].extra += p[pj].notes.len() - c;
        }
    }
    for &ei in &missed_chords {
        if covered.contains(&ei)
            && let Some(s) = sec(e[ei].t)
        {
            sections[s].missed += e[ei].notes.len();
        }
    }
    for &pj in &extra_chords {
        // colocar o acorde a mais no tempo original estimado
        let t = if a.abs() > 1e-9 { (p[pj].t - b) / a } else { p[pj].t };
        if let Some(s) = sec(t) {
            sections[s].extra += p[pj].notes.len();
        }
    }
    // Só os compassos do trecho coberto interessam
    let (cs, ce) = (e[first_e].t, e[last_e].t);
    let sections: Vec<SectionStats> = sections
        .into_iter()
        .enumerate()
        .filter(|(i, s)| {
            let next = bars.get(i + 1).map(|d| d.as_secs_f64()).unwrap_or(f64::MAX);
            next > cs && s.start.as_secs_f64() <= ce
        })
        .map(|(_, s)| s)
        .collect();

    Alignment {
        report: Report {
            expected: expected_notes,
            hits,
            missed,
            extra,
            accuracy: if expected_notes == 0 { 0.0 } else { hits as f32 / expected_notes as f32 },
            mean_offset_ms: 0.0,
            offset_spread_ms: spread_ms as f32,
            hesitations,
            sections,
        },
        tempo_ratio: if a > 1e-9 { (1.0 / a) as f32 } else { 1.0 },
        covered_start: Duration::from_secs_f64(cs),
        covered_end: Duration::from_secs_f64(ce),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Melodia de teste com acordes: (tempo em ms, notas)
    fn piece() -> Vec<ExpectedNote> {
        let mut v = Vec::new();
        for k in 0..40u64 {
            let t = Duration::from_millis(k * 500);
            v.push(ExpectedNote { note: 60 + (k % 12) as u8, start: t });
            if k % 4 == 0 {
                v.push(ExpectedNote { note: 48 + (k % 7) as u8, start: t });
            }
        }
        v
    }

    fn play_from(exp: &[ExpectedNote], stretch: f64, offset_ms: u64) -> Vec<PlayedNote> {
        exp.iter()
            .map(|n| {
                let t = Duration::from_secs_f64(n.start.as_secs_f64() * stretch)
                    + Duration::from_millis(offset_ms);
                PlayedNote { note: n.note, start: t, end: t + Duration::from_millis(200), velocity: 80 }
            })
            .collect()
    }

    #[test]
    fn slower_and_offset_take_is_perfect() {
        let e = piece();
        let p = play_from(&e, 1.25, 4000); // 25 % mais lento, 4 s de silêncio
        let a = align(&e, &p, &[]);
        assert_eq!(a.report.accuracy, 1.0, "{:?}", a.report);
        assert_eq!(a.report.extra, 0);
        assert!((a.tempo_ratio - 0.8).abs() < 0.01, "{}", a.tempo_ratio);
        assert!(a.report.offset_spread_ms < 1.0);
    }

    #[test]
    fn partial_take_from_the_middle_counts_only_covered_part() {
        let e = piece();
        let middle: Vec<ExpectedNote> = e
            .iter()
            .filter(|n| n.start >= Duration::from_secs(5) && n.start < Duration::from_secs(12))
            .cloned()
            .collect();
        let p = play_from(&middle, 1.0, 0);
        let a = align(&e, &p, &[]);
        assert_eq!(a.report.accuracy, 1.0);
        assert_eq!(a.report.expected, middle.len());
        assert_eq!(a.covered_start, Duration::from_secs(5));
    }

    #[test]
    fn wrong_and_missing_notes_are_counted() {
        let e = piece();
        let mut p = play_from(&e, 1.0, 0);
        p.remove(10); // falha uma nota
        p[20].note += 1; // uma nota errada
        let a = align(&e, &p, &[]);
        assert_eq!(a.report.hits, e.len() - 2);
        assert_eq!(a.report.missed, 2);
        assert_eq!(a.report.extra, 1);
    }

    #[test]
    fn chord_note_order_does_not_matter() {
        let e = vec![
            ExpectedNote { note: 60, start: Duration::ZERO },
            ExpectedNote { note: 64, start: Duration::ZERO },
            ExpectedNote { note: 67, start: Duration::ZERO },
        ];
        let p = vec![
            PlayedNote { note: 67, start: Duration::from_millis(0), end: Duration::from_millis(100), velocity: 80 },
            PlayedNote { note: 60, start: Duration::from_millis(20), end: Duration::from_millis(100), velocity: 80 },
            PlayedNote { note: 64, start: Duration::from_millis(40), end: Duration::from_millis(100), velocity: 80 },
        ];
        assert_eq!(align(&e, &p, &[]).report.hits, 3);
    }
}

/// Ferramenta de desenvolvimento: compara ficheiros reais.
/// `NEOTHASIA_ALIGN_SONG=orig.mid NEOTHASIA_ALIGN_DIR=pasta cargo test -p neothesia real_files -- --ignored --nocapture`
#[cfg(test)]
mod real_files {
    use super::*;

    pub fn notes_of(path: &std::path::Path) -> (Vec<ExpectedNote>, Vec<PlayedNote>, Vec<Duration>) {
        let midi = midi_file::MidiFile::new(path).unwrap();
        let mut e = Vec::new();
        let mut p = Vec::new();
        for t in midi.tracks.iter() {
            for n in t.notes.iter() {
                e.push(ExpectedNote { note: n.note, start: n.start });
                p.push(PlayedNote { note: n.note, start: n.start, end: n.end, velocity: n.velocity });
            }
        }
        (e, p, midi.measures.to_vec())
    }

    #[test]
    #[ignore]
    fn real_files() {
        let song = std::env::var("NEOTHASIA_ALIGN_SONG").unwrap();
        let dir = std::env::var("NEOTHASIA_ALIGN_DIR").unwrap();
        let (exp, _, bars) = notes_of(std::path::Path::new(&song));
        let mut files: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.path()).collect();
        files.sort();
        for f in files.iter().filter(|f| f.extension().is_some_and(|x| x == "mid")) {
            let (_, played, _) = notes_of(f);
            let start = std::time::Instant::now();
            let a = align(&exp, &played, &bars);
            println!(
                "{:<34} {:>4} notas | {:>5.1}% certas ({} de {}) | {} a mais | andamento {:>3.0}% | regularidade {:>4.0} ms | {} hesitações | trecho {:>5.1}s–{:>5.1}s | {} compassos | {:?}",
                f.file_name().unwrap().to_string_lossy(),
                played.len(),
                a.report.accuracy * 100.0,
                a.report.hits,
                a.report.expected,
                a.report.extra,
                a.tempo_ratio * 100.0,
                a.report.offset_spread_ms,
                a.report.hesitations,
                a.covered_start.as_secs_f32(),
                a.covered_end.as_secs_f32(),
                a.report.sections.len(),
                start.elapsed(),
            );
        }
    }
}
