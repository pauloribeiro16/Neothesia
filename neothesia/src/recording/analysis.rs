//! Análise detalhada de uma gravação contra o original, por compasso e por nota.
//!
//! Parte do alinhamento (`align`), que dá os acordes emparelhados, e constrói:
//! - um **mapa de tempo** original ↔ gravação (linear por troços);
//! - o emparelhamento **nota a nota** (apanha acordes "espalhados" que o
//!   agrupamento por acordes separaria);
//! - por compasso: andamento local, acordes não simultâneos, hesitações, erros e
//!   uma nota de qualidade (0..1) para o mapa de compassos;
//! - **sugestões** em português ("Nos compassos 17–24 a mão esquerda entra…").
//!
//! Tudo em segundos (`f64`). Tempo "original" = linha do tempo do MIDI da música.

use std::time::Duration;

use super::{ExpectedNote, PlayedNote, align::align};

/// Acorde "espalhado" a partir deste desvio entre a primeira e a última nota.
pub const SPREAD_WARN_MS: f32 = 40.0;
/// Nota fora de tempo (em relação ao andamento local) a partir deste desvio.
pub const NOTE_WARN_MS: f32 = 60.0;
/// Pausa para pensar/corrigir.
const HESITATION_S: f64 = 0.35;
/// Janela (em acordes) do andamento local.
const LOCAL_WINDOW: usize = 8;
/// Procura de cada nota esperada à volta do tempo previsto na gravação.
const NOTE_SEARCH_S: f64 = 0.30;
/// Notas esperadas que começam a menos disto contam como o mesmo acorde.
const CHORD_S: f64 = 0.03;

/// Mapa de tempo monótono entre o original e a gravação.
#[derive(Debug, Clone, Default)]
pub struct TimeMap {
    pts: Vec<(f64, f64)>, // (original, gravação), crescente nos dois
}

impl TimeMap {
    pub fn new(anchors: &[(f64, f64)]) -> Self {
        let mut pts: Vec<(f64, f64)> = Vec::with_capacity(anchors.len());
        for &(x, y) in anchors {
            match pts.last() {
                Some(&(lx, ly)) if x <= lx || y <= ly => {}
                _ => pts.push((x, y)),
            }
        }
        Self { pts }
    }

    pub fn is_empty(&self) -> bool {
        self.pts.len() < 2
    }

    fn interp(pts: &[(f64, f64)], x: f64, fwd: bool) -> f64 {
        let key = |p: &(f64, f64)| if fwd { p.0 } else { p.1 };
        let val = |p: &(f64, f64)| if fwd { p.1 } else { p.0 };
        if pts.is_empty() {
            return x;
        }
        if pts.len() == 1 {
            return x - key(&pts[0]) + val(&pts[0]);
        }
        let i = pts.partition_point(|p| key(p) < x).clamp(1, pts.len() - 1);
        let (a, b) = (&pts[i - 1], &pts[i]);
        let t = (x - key(a)) / (key(b) - key(a));
        val(a) + t * (val(b) - val(a))
    }

    /// Tempo na gravação correspondente a um tempo do original.
    pub fn to_played(&self, orig: f64) -> f64 {
        Self::interp(&self.pts, orig, true)
    }

    /// Tempo no original correspondente a um tempo da gravação.
    pub fn to_orig(&self, played: f64) -> f64 {
        Self::interp(&self.pts, played, false)
    }
}

/// Uma nota esperada e o que foi tocado para ela.
#[derive(Debug, Clone, PartialEq)]
pub struct NoteMatch {
    pub note: u8,
    /// Início e fim no original
    pub start: f64,
    pub end: f64,
    /// Índice da nota tocada correspondente (em `Analysis::played`), se houver
    pub played: Option<usize>,
    /// Desvio em ms do início tocado em relação ao andamento local (> 0 = atrasado)
    pub deviation_ms: Option<f32>,
}

/// Uma nota tocada, já colocada na linha do tempo do original.
#[derive(Debug, Clone, PartialEq)]
pub struct PlacedNote {
    pub note: u8,
    /// Início e fim convertidos para o tempo do original
    pub start: f64,
    pub end: f64,
    pub velocity: u8,
    /// Corresponde a uma nota esperada?
    pub matched: bool,
    /// Fora de tempo (|desvio| > `NOTE_WARN_MS`)?
    pub off_time: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct BarStats {
    /// Início e fim do compasso no original
    pub start: f64,
    pub end: f64,
    pub hits: usize,
    pub missed: usize,
    pub extra: usize,
    /// Andamento local (velocidade do utilizador / original), se tocado
    pub tempo: Option<f32>,
    /// Média do espalhamento dos acordes (ms) — notas que deviam soar juntas
    pub chord_spread_ms: f32,
    /// > 0: o baixo (mão esquerda) entra depois da melodia, em ms (média)
    pub bass_late_ms: f32,
    pub hesitations: usize,
    /// Qualidade 0..1 (1 = certo e a tempo). `None` = fora do trecho tocado.
    pub score: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InsightKind {
    ChordSpread,
    SlowDown,
    Hesitation,
    Strength,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Insight {
    pub kind: InsightKind,
    /// Compassos (1-based, inclusivo)
    pub bars: (usize, usize),
    pub text: String,
}

impl Insight {
    pub fn is_positive(&self) -> bool {
        self.kind == InsightKind::Strength
    }
}

#[derive(Debug, Clone, Default)]
pub struct Analysis {
    pub map: TimeMap,
    pub notes: Vec<NoteMatch>,
    pub played: Vec<PlacedNote>,
    pub bars: Vec<BarStats>,
    pub insights: Vec<Insight>,
    /// Andamento médio (velocidade do utilizador / original)
    pub tempo: f32,
}

impl Analysis {
    /// Compasso (0-based) que contém o tempo `t` do original.
    pub fn bar_at(&self, t: f64) -> Option<usize> {
        if self.bars.is_empty() {
            return None;
        }
        Some(self.bars.partition_point(|b| b.start <= t).saturating_sub(1))
    }
}

fn local_slope(xs: &[f64], ys: &[f64], i: usize) -> f64 {
    let lo = i.saturating_sub(LOCAL_WINDOW);
    let hi = (i + LOCAL_WINDOW + 1).min(xs.len());
    let (wx, wy) = (&xs[lo..hi], &ys[lo..hi]);
    let k = wx.len() as f64;
    let (mx, my) = (wx.iter().sum::<f64>() / k, wy.iter().sum::<f64>() / k);
    let sxx: f64 = wx.iter().map(|x| (x - mx).powi(2)).sum();
    let sxy: f64 = wx.iter().zip(wy).map(|(x, y)| (x - mx) * (y - my)).sum();
    if sxx > 1e-9 { sxy / sxx } else { 1.0 }
}

pub fn analyze(expected: &[ExpectedNote], played_in: &[PlayedNote], bars_in: &[Duration]) -> Analysis {
    let a = align(expected, played_in, bars_in);
    let map = TimeMap::new(&a.anchors);
    if map.is_empty() {
        return Analysis { map, tempo: 1.0, ..Default::default() };
    }
    let (cov_s, cov_e) = (a.covered_start.as_secs_f64(), a.covered_end.as_secs_f64());

    // ── Notas esperadas (só o trecho coberto) e tocadas ───────────────────
    let mut exp: Vec<ExpectedNote> = expected
        .iter()
        .filter(|n| {
            let t = n.start.as_secs_f64();
            t >= cov_s - 1e-6 && t <= cov_e + 1e-6
        })
        .cloned()
        .collect();
    exp.sort_by_key(|n| (n.start, n.note));
    let mut played: Vec<PlayedNote> = played_in.to_vec();
    played.sort_by_key(|n| (n.start, n.note));
    let ps: Vec<f64> = played.iter().map(|n| n.start.as_secs_f64()).collect();

    // ── Emparelhamento nota a nota, guiado pelo mapa de tempo ─────────────
    let mut used = vec![false; played.len()];
    let mut notes: Vec<NoteMatch> = Vec::with_capacity(exp.len());
    // Fim de cada nota esperada: procurar no ficheiro original não é preciso aqui;
    // usamos 1/4 s como mínimo visual quando não há fim conhecido.
    for e in &exp {
        let t = e.start.as_secs_f64();
        let pred = map.to_played(t);
        let from = ps.partition_point(|&s| s < pred - NOTE_SEARCH_S);
        let mut best: Option<(usize, f64)> = None;
        for (j, &s) in ps.iter().enumerate().skip(from) {
            if s > pred + NOTE_SEARCH_S {
                break;
            }
            if used[j] || played[j].note != e.note {
                continue;
            }
            let d = (s - pred).abs();
            if best.is_none_or(|(_, bd)| d < bd) {
                best = Some((j, d));
            }
        }
        if let Some((j, _)) = best {
            used[j] = true;
        }
        notes.push(NoteMatch {
            note: e.note,
            start: t,
            end: t + 0.25,
            played: best.map(|b| b.0),
            deviation_ms: None,
        });
    }

    // ── Andamento local e desvio de cada nota ─────────────────────────────
    let (xs, ys): (Vec<f64>, Vec<f64>) = a.anchors.iter().cloned().unzip();
    let slope_at = |t: f64| -> f64 {
        let i = xs.partition_point(|&x| x < t).min(xs.len() - 1);
        local_slope(&xs, &ys, i)
    };
    for n in &mut notes {
        if let Some(j) = n.played {
            // Previsão local: passa pela âncora mais próxima com a inclinação local
            let i = xs.partition_point(|&x| x < n.start).min(xs.len() - 1);
            let i = if i > 0 && (n.start - xs[i - 1]).abs() < (xs[i] - n.start).abs() { i - 1 } else { i };
            let pred = ys[i] + slope_at(n.start) * (n.start - xs[i]);
            n.deviation_ms = Some(((ps[j] - pred) * 1000.0) as f32);
        }
    }

    // ── Notas tocadas na linha do tempo do original ───────────────────────
    let mut matched_dev: Vec<Option<f32>> = vec![None; played.len()];
    for n in &notes {
        if let (Some(j), Some(d)) = (n.played, n.deviation_ms) {
            matched_dev[j] = Some(d);
        }
    }
    let placed: Vec<PlacedNote> = played
        .iter()
        .enumerate()
        .map(|(j, p)| {
            let s = map.to_orig(p.start.as_secs_f64());
            let e = map.to_orig(p.end.as_secs_f64()).max(s + 0.02);
            PlacedNote {
                note: p.note,
                start: s,
                end: e,
                velocity: p.velocity,
                matched: used[j],
                off_time: matched_dev[j].is_some_and(|d| d.abs() > NOTE_WARN_MS),
            }
        })
        .collect();

    // ── Compassos ─────────────────────────────────────────────────────────
    let bar_starts: Vec<f64> = if bars_in.is_empty() {
        vec![0.0]
    } else {
        bars_in.iter().map(|d| d.as_secs_f64()).collect()
    };
    let last_t = expected.iter().map(|n| n.start.as_secs_f64()).fold(0.0, f64::max) + 0.5;
    let mut bars: Vec<BarStats> = bar_starts
        .iter()
        .enumerate()
        .map(|(i, &s)| BarStats {
            start: s,
            end: bar_starts.get(i + 1).cloned().unwrap_or(last_t.max(s + 0.5)),
            ..Default::default()
        })
        .collect();
    let bar_of = |t: f64| bar_starts.partition_point(|&b| b <= t).saturating_sub(1);

    for n in &notes {
        let b = bar_of(n.start);
        if n.played.is_some() {
            bars[b].hits += 1;
        } else {
            bars[b].missed += 1;
        }
    }
    for p in placed.iter().filter(|p| !p.matched) {
        if p.start >= cov_s - 0.5 && p.start <= cov_e + 0.5 {
            bars[bar_of(p.start)].extra += 1;
        }
    }

    // Acordes esperados → espalhamento e "baixo atrasado"
    let mut spreads: Vec<Vec<(f32, f32)>> = vec![Vec::new(); bars.len()];
    let mut i = 0;
    while i < notes.len() {
        let mut k = i + 1;
        while k < notes.len() && notes[k].start - notes[i].start <= CHORD_S {
            k += 1;
        }
        let chord: Vec<(u8, f64)> = notes[i..k]
            .iter()
            .filter_map(|n| n.played.map(|j| (n.note, ps[j])))
            .collect();
        if chord.len() >= 2 {
            let min = chord.iter().map(|c| c.1).fold(f64::MAX, f64::min);
            let max = chord.iter().map(|c| c.1).fold(f64::MIN, f64::max);
            let low = chord.iter().min_by_key(|c| c.0).unwrap().1;
            let high = chord.iter().max_by_key(|c| c.0).unwrap().1;
            spreads[bar_of(notes[i].start)].push((((max - min) * 1000.0) as f32, ((low - high) * 1000.0) as f32));
        }
        i = k;
    }
    for (b, sp) in spreads.iter().enumerate() {
        if !sp.is_empty() {
            bars[b].chord_spread_ms = sp.iter().map(|s| s.0).sum::<f32>() / sp.len() as f32;
            bars[b].bass_late_ms = sp.iter().map(|s| s.1).sum::<f32>() / sp.len() as f32;
        }
    }

    // Andamento local por compasso (inclinação do mapa dentro do compasso)
    for b in &mut bars {
        let (s, e) = (b.start.max(cov_s), b.end.min(cov_e));
        if e - s > 0.2 {
            let d = map.to_played(e) - map.to_played(s);
            if d > 1e-6 {
                b.tempo = Some(((e - s) / d) as f32);
            }
        }
    }

    // Hesitações: intervalos entre acordes muito maiores do que o andamento local prevê
    for idx in 1..xs.len() {
        let slope = local_slope(&xs, &ys, idx);
        let r = (ys[idx] - ys[idx - 1]) - slope * (xs[idx] - xs[idx - 1]);
        if r > HESITATION_S {
            let b = bar_of(xs[idx]);
            bars[b].hesitations += 1;
        }
    }

    // Andamento médio e qualidade de cada compasso
    let mut tempos: Vec<f32> = bars.iter().filter_map(|b| b.tempo).collect();
    tempos.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median_tempo = tempos.get(tempos.len() / 2).cloned().unwrap_or(a.tempo_ratio);
    for b in &mut bars {
        let total = b.hits + b.missed;
        if total == 0 && b.extra == 0 {
            continue;
        }
        let hit_rate = if total == 0 { 0.0 } else { b.hits as f32 / total as f32 };
        let extra_pen = (b.extra as f32 / (total.max(1) as f32)).min(1.0) * 0.5;
        let spread_pen = (b.chord_spread_ms / 150.0).min(1.0) * 0.4;
        let hes_pen = (b.hesitations as f32 * 0.25).min(0.5);
        let tempo_pen = b
            .tempo
            .map(|t| ((median_tempo - t) / median_tempo).max(0.0) * 0.8)
            .unwrap_or(0.0)
            .min(0.4);
        b.score = Some((hit_rate - extra_pen - spread_pen - hes_pen - tempo_pen).clamp(0.0, 1.0));
    }

    let insights = make_insights(&bars, median_tempo);

    Analysis {
        map,
        notes,
        played: placed,
        bars,
        insights,
        tempo: median_tempo,
    }
}

/// Atalho para a UI: carrega a música e a gravação do disco e analisa.
/// Usa as notas de todas as faixas da música (como na importação).
pub fn analyze_files(song: &std::path::Path, take_midi: &std::path::Path) -> Result<Analysis, String> {
    let orig = midi_file::MidiFile::new(song)?;
    let take = midi_file::MidiFile::new(take_midi)?;
    let expected: Vec<ExpectedNote> = orig
        .tracks
        .iter()
        .flat_map(|t| t.notes.iter())
        .map(|n| ExpectedNote { note: n.note, start: n.start })
        .collect();
    let played: Vec<PlayedNote> = take
        .tracks
        .iter()
        .flat_map(|t| t.notes.iter())
        .map(|n| PlayedNote { note: n.note, start: n.start, end: n.end, velocity: n.velocity })
        .collect();
    Ok(analyze(&expected, &played, &orig.measures))
}

/// Junta compassos seguidos que cumprem `pred` em intervalos (0-based, inclusivo).
fn runs(bars: &[BarStats], pred: impl Fn(&BarStats) -> bool) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    for (i, b) in bars.iter().enumerate() {
        match (pred(b), start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                out.push((s, i - 1));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        out.push((s, bars.len() - 1));
    }
    out
}

fn bars_label(r: (usize, usize)) -> String {
    if r.0 == r.1 {
        format!("No compasso {}", r.0 + 1)
    } else {
        format!("Nos compassos {}–{}", r.0 + 1, r.1 + 1)
    }
}

fn make_insights(bars: &[BarStats], median_tempo: f32) -> Vec<Insight> {
    let mut out = Vec::new();
    let played: Vec<(usize, &BarStats)> = bars.iter().enumerate().filter(|(_, b)| b.score.is_some()).collect();

    // 1a. Padrão geral: baixo sistematicamente atrasado/adiantado em relação à melodia
    let with_chords: Vec<f32> = played.iter().filter(|(_, b)| b.chord_spread_ms > 0.0).map(|(_, b)| b.bass_late_ms).collect();
    let mut general_hand = false;
    if with_chords.len() >= 6 {
        let mut sorted = with_chords.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median = sorted[sorted.len() / 2];
        let late = with_chords.iter().filter(|&&v| v > 10.0).count();
        let early = with_chords.iter().filter(|&&v| v < -10.0).count();
        let (first, last) = (played.first().unwrap().0 + 1, played.last().unwrap().0 + 1);
        if median > 12.0 && late * 3 >= with_chords.len() * 2 {
            general_hand = true;
            out.push(Insight {
                kind: InsightKind::ChordSpread,
                bars: (first, last),
                text: format!(
                    "Em {late} de {} compassos a mão esquerda entra depois da melodia (em média {median:.0} ms). Tenta pousar as duas mãos juntas.",
                    with_chords.len()
                ),
            });
        } else if median < -12.0 && early * 3 >= with_chords.len() * 2 {
            general_hand = true;
            out.push(Insight {
                kind: InsightKind::ChordSpread,
                bars: (first, last),
                text: format!(
                    "Em {early} de {} compassos a melodia entra depois do baixo (em média {:.0} ms).",
                    with_chords.len(),
                    -median
                ),
            });
        }
    }

    // 1b. Trechos com acordes muito espalhados (os 2 piores; 1 se já há o padrão geral)
    let mut spread_runs: Vec<((usize, usize), f32, f32)> = runs(bars, |b| b.chord_spread_ms >= SPREAD_WARN_MS)
        .into_iter()
        .map(|r| {
            let sl = &bars[r.0..=r.1];
            let n = sl.len() as f32;
            (r, sl.iter().map(|b| b.chord_spread_ms).sum::<f32>() / n, sl.iter().map(|b| b.bass_late_ms).sum::<f32>() / n)
        })
        .collect();
    spread_runs.sort_by(|a, b| (((b.0.1 - b.0.0) + 1) as f32 * b.1).partial_cmp(&(((a.0.1 - a.0.0) + 1) as f32 * a.1)).unwrap());
    for (r, spread, bass) in spread_runs.into_iter().take(if general_hand { 1 } else { 2 }) {
        let who = if bass > 15.0 {
            format!("a mão esquerda entra em média {bass:.0} ms depois da melodia")
        } else if bass < -15.0 {
            format!("a melodia entra em média {:.0} ms depois do baixo", -bass)
        } else {
            format!("as notas dos acordes espalham-se em média {spread:.0} ms")
        };
        out.push(Insight { kind: InsightKind::ChordSpread, bars: (r.0 + 1, r.1 + 1), text: format!("{} {}.", bars_label(r), who) });
    }

    // 2a. Secção mais lenta: média móvel de 4 compassos < 90 % do andamento típico
    let tempos: Vec<Option<f32>> = bars.iter().map(|b| b.tempo).collect();
    let rolling: Vec<Option<f32>> = (0..bars.len())
        .map(|i| {
            let w: Vec<f32> = tempos[i.saturating_sub(1)..(i + 3).min(bars.len())].iter().flatten().cloned().collect();
            (w.len() >= 2).then(|| w.iter().sum::<f32>() / w.len() as f32)
        })
        .collect();
    let slow_secs: Vec<(usize, usize)> = {
        let mut v = Vec::new();
        let mut start = None;
        for (i, r) in rolling.iter().enumerate() {
            let slow = r.is_some_and(|t| t < median_tempo * 0.9);
            match (slow, start) {
                (true, None) => start = Some(i),
                (false, Some(s)) => {
                    v.push((s, i - 1));
                    start = None;
                }
                _ => {}
            }
        }
        if let Some(s) = start {
            v.push((s, bars.len() - 1));
        }
        v
    };
    let mut slow_reported = None;
    if let Some(&r) = slow_secs.iter().filter(|r| r.1 - r.0 >= 3).max_by_key(|r| r.1 - r.0) {
        let inside: Vec<f32> = bars[r.0..=r.1].iter().filter_map(|b| b.tempo).collect();
        let before: Vec<f32> = bars[..r.0].iter().filter_map(|b| b.tempo).collect();
        let mean = |v: &[f32]| v.iter().sum::<f32>() / v.len().max(1) as f32;
        let text = if before.len() >= 3 {
            format!(
                "A partir do compasso {} abrandas: de {:.0}% para {:.0}% do original (até ao compasso {}).",
                r.0 + 1,
                mean(&before) * 100.0,
                mean(&inside) * 100.0,
                r.1 + 1
            )
        } else {
            format!("{} tocas mais devagar: {:.0}% do original.", bars_label(r), mean(&inside) * 100.0)
        };
        out.push(Insight { kind: InsightKind::SlowDown, bars: (r.0 + 1, r.1 + 1), text });
        slow_reported = Some(r);
    }

    // 2b. Quebra pontual de andamento (> 15 % abaixo), fora da secção já referida
    let dips = runs(bars, |b| b.tempo.is_some_and(|t| t < median_tempo * 0.75));
    if let Some(&r) = dips
        .iter()
        .filter(|r| slow_reported.is_none_or(|s| r.0 < s.0 || r.1 > s.1) || slow_reported.is_some())
        .min_by(|a, b| {
            let m = |r: &(usize, usize)| bars[r.0..=r.1].iter().filter_map(|b| b.tempo).fold(f32::MAX, f32::min);
            m(a).partial_cmp(&m(b)).unwrap()
        })
    {
        let min = bars[r.0..=r.1].iter().filter_map(|b| b.tempo).fold(f32::MAX, f32::min);
        out.push(Insight {
            kind: InsightKind::SlowDown,
            bars: (r.0 + 1, r.1 + 1),
            text: format!("{} o andamento cai para {:.0}% — vale a pena praticar esse trecho devagar.", bars_label(r), min * 100.0),
        });
    }

    // 3. Hesitações
    let hes: Vec<usize> = bars.iter().enumerate().filter(|(_, b)| b.hesitations > 0).map(|(i, _)| i + 1).collect();
    if !hes.is_empty() {
        let total: usize = bars.iter().map(|b| b.hesitations).sum();
        let list = match hes.len() {
            1 => hes[0].to_string(),
            _ => {
                let (last, rest) = hes.split_last().unwrap();
                format!("{} e {}", rest.iter().map(|b| b.to_string()).collect::<Vec<_>>().join(", "), last)
            }
        };
        let what = if total == 1 { "A hesitação está".to_string() } else { format!("As {total} hesitações estão") };
        let place = if hes.len() == 1 { "no compasso" } else { "nos compassos" };
        out.push(Insight { kind: InsightKind::Hesitation, bars: (hes[0], *hes.last().unwrap()), text: format!("{what} {place} {list}.") });
    }

    // 4. Ponto forte: maior sequência de compassos bons (≥ 4)
    let good = runs(bars, |b| b.score.is_some_and(|s| s >= 0.85));
    if let Some(&r) = good.iter().max_by_key(|r| r.1 - r.0)
        && r.1 - r.0 >= 3
    {
        out.push(Insight {
            kind: InsightKind::Strength,
            bars: (r.0 + 1, r.1 + 1),
            text: format!("Os compassos {}–{} estão quase perfeitos.", r.0 + 1, r.1 + 1),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(ms: u64) -> Duration {
        Duration::from_millis(ms)
    }

    /// 16 compassos de 3 tempos (500 ms cada): melodia + acorde de 2 notas no 2.º e 3.º tempos
    fn piece() -> (Vec<ExpectedNote>, Vec<Duration>) {
        let mut e = Vec::new();
        let mut bars = Vec::new();
        for bar in 0..16u64 {
            bars.push(d(bar * 1500));
            for beat in 0..3u64 {
                let t = d(bar * 1500 + beat * 500);
                e.push(ExpectedNote { note: 72 + ((bar + beat) % 5) as u8, start: t });
                if beat > 0 {
                    e.push(ExpectedNote { note: 48 + (bar % 3) as u8, start: t });
                    e.push(ExpectedNote { note: 55 + (bar % 3) as u8, start: t });
                }
            }
        }
        (e, bars)
    }

    /// Toca a peça a `ratio` da velocidade, com o baixo `bass_late` ms atrasado nos
    /// compassos `late_bars`, e uma pausa extra de `pause` ms antes do compasso `pause_bar`.
    fn play(e: &[ExpectedNote], ratio: f64, late_bars: std::ops::Range<u64>, bass_late: u64, pause_bar: u64, pause: u64) -> Vec<PlayedNote> {
        e.iter()
            .map(|n| {
                let bar = n.start.as_millis() as u64 / 1500;
                let mut t = (n.start.as_secs_f64() / ratio * 1000.0) as u64 + 2000;
                if n.note < 60 && late_bars.contains(&bar) {
                    t += bass_late;
                }
                if bar >= pause_bar {
                    t += pause;
                }
                PlayedNote { note: n.note, start: d(t), end: d(t + 300), velocity: 70 }
            })
            .collect()
    }

    #[test]
    fn time_map_roundtrip() {
        let m = TimeMap::new(&[(0.0, 2.0), (1.0, 3.5), (2.0, 5.0)]);
        assert!((m.to_played(1.5) - 4.25).abs() < 1e-9);
        assert!((m.to_orig(4.25) - 1.5).abs() < 1e-9);
    }

    #[test]
    fn detects_late_bass_slowdown_and_hesitation() {
        let (e, bars) = piece();
        let p = play(&e, 0.65, 4..8, 90, 12, 700);
        let a = analyze(&e, &p, &bars);

        assert!((a.tempo - 0.65).abs() < 0.05, "andamento {}", a.tempo);
        // Compassos 5–8 (1-based) com acordes espalhados e baixo atrasado
        for b in 4..8 {
            assert!(a.bars[b].chord_spread_ms > 60.0, "compasso {} spread {}", b + 1, a.bars[b].chord_spread_ms);
            assert!(a.bars[b].bass_late_ms > 60.0);
        }
        assert!(a.bars[1].chord_spread_ms < 10.0);
        // Pausa antes do compasso 13 → hesitação aí
        assert!(a.bars[12].hesitations >= 1, "{:?}", a.bars[12]);
        // Todas as notas encontradas (mesmo as do baixo atrasado 90 ms)
        assert!(a.notes.iter().all(|n| n.played.is_some()));

        let texts: Vec<&str> = a.insights.iter().map(|i| i.text.as_str()).collect();
        assert!(texts.iter().any(|t| t.contains("compassos 5–8") && t.contains("mão esquerda")), "{texts:?}");
        assert!(texts.iter().any(|t| t.contains("hesitação")), "{texts:?}");
        assert!(a.insights.iter().any(|i| i.is_positive()), "{texts:?}");
    }

    #[test]
    fn played_notes_land_on_original_timeline() {
        let (e, bars) = piece();
        let p = play(&e, 0.5, 0..0, 0, 99, 0);
        let a = analyze(&e, &p, &bars);
        // A 1.ª nota tocada (2 s de silêncio, metade da velocidade) cai em t≈0 no original
        let first = a.played.iter().map(|n| n.start).fold(f64::MAX, f64::min);
        assert!(first.abs() < 0.02, "{first}");
        assert!(a.played.iter().all(|n| n.matched && !n.off_time));
    }
}

/// Ferramenta de desenvolvimento com ficheiros reais:
/// `NEOTHASIA_ALIGN_SONG=orig.mid NEOTHASIA_TAKE=gravacao.mid cargo test -p neothesia analysis_real -- --ignored --nocapture`
#[cfg(test)]
mod real {
    use super::*;

    fn load(p: &str) -> (Vec<ExpectedNote>, Vec<PlayedNote>, Vec<Duration>) {
        let m = midi_file::MidiFile::new(p).unwrap();
        let mut e = Vec::new();
        let mut pl = Vec::new();
        for t in m.tracks.iter() {
            for n in t.notes.iter() {
                e.push(ExpectedNote { note: n.note, start: n.start });
                pl.push(PlayedNote { note: n.note, start: n.start, end: n.end, velocity: n.velocity });
            }
        }
        (e, pl, m.measures.to_vec())
    }

    #[test]
    #[ignore]
    fn analysis_real() {
        let (e, _, bars) = load(&std::env::var("NEOTHASIA_ALIGN_SONG").unwrap());
        let (_, p, _) = load(&std::env::var("NEOTHASIA_TAKE").unwrap());
        let t0 = std::time::Instant::now();
        let a = analyze(&e, &p, &bars);
        println!("análise em {:?} · andamento típico {:.0}%", t0.elapsed(), a.tempo * 100.0);
        for (i, b) in a.bars.iter().enumerate().filter(|(_, b)| b.score.is_some()) {
            println!(
                "c{:>3} {:>5.1}s  certas {:>2} falh {:>2} mais {:>2} | andamento {:>4} | acordes {:>4.0} ms (baixo {:>+4.0}) | hes {} | nota {:.2}",
                i + 1, b.start, b.hits, b.missed, b.extra,
                b.tempo.map(|t| format!("{:.0}%", t * 100.0)).unwrap_or("-".into()),
                b.chord_spread_ms, b.bass_late_ms, b.hesitations, b.score.unwrap()
            );
        }
        println!("--- sugestões ---");
        for i in &a.insights {
            println!("[{:?}] {}", i.kind, i.text);
        }
    }
}
