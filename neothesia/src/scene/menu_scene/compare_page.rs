//! Página "Comparação" (R6, 3.º ecrã da maqueta): os números da gravação, a
//! curva de andamento ao longo da peça, o mapa de compassos e o piano roll de
//! 4 compassos com as notas esperadas (contorno) e tocadas (cheio), mais a
//! coluna "O que melhorar" com as sugestões da análise.
//!
//! Dados: `self.state.compare_song` (música), `self.state.compare_take`
//! (gravação) e `self.state.compare` (`Analysis`, notas já no tempo do
//! original). Vem de `UiState::open_compare`.

use std::path::PathBuf;

use nuon::TextAlign;

use crate::{
    context::Context,
    icons,
    library::Library,
    locals,
    recording::analysis::{Analysis, InsightKind},
    theme,
    widgets,
};

/// Glifo para "Comparar" nas linhas de gravação (bar-chart-steps)
pub const COMPARE_ICON: &str = "\u{F17B}";

/// Compassos visíveis no piano roll
const ZOOM_BARS: usize = 4;
/// Espalhamento de acorde a partir do qual se assinala (ms)
const CHORD_WARN_MS: f32 = 40.0;

enum Act {
    Back,
    ViewTake,
}

/// Estado local da página
#[derive(Default)]
struct PageState {
    /// Primeiro compasso do zoom (0-based)
    zoom_start: usize,
}

fn page_state(ui: &mut nuon::Ui) -> &mut PageState {
    ui.data::<PageState>(widgets::wid(("compare", "page_state")))
}

impl super::MenuScene {
    pub fn compare_page_window_event(
        &mut self,
        _ctx: &mut Context,
        event: &winit::event::WindowEvent,
    ) {
        use crate::utils::window::WinitEvent;
        use winit::keyboard::{Key, NamedKey};
        if event.key_pressed(Key::Named(NamedKey::Escape)) {
            self.state.go_back();
        }
    }

    pub fn compare_page_ui(&mut self, ctx: &mut Context, ui: &mut nuon::Ui) {
        let win_h = ctx.window_state.logical_size.height;

        let (nav, nav_act) = self.side_nav(ctx, ui);
        let mut act: Option<Act> = None;

        let Some(song) = self.state.compare_song.clone() else {
            self.state.go_back();
            return;
        };
        let Some(take) = self.state.compare_take.clone() else {
            self.state.go_back();
            return;
        };
        let Some(analysis) = self.state.compare.clone() else {
            self.state.go_back();
            return;
        };

        let body_w = nav.content_w - theme::SPACE_XL * 2.0;
        let x0 = nav.content_x + theme::SPACE_XL;

        // ── Cabeçalho: música + gravação ─────────────────────────────
        let (title, composer) = Library::pretty_title(&song);
        nuon::translate().x(x0).y(theme::SPACE_S).build(ui, |ui| {
            widgets::page_title(ui, body_w, &title);
            let sub = format!(
                "{}{} · {} · {:.0} %",
                composer.map(|c| format!("{c} · ")).unwrap_or_default(),
                take.date_label(),
                locals::song::compare_best(),
                take.accuracy * 100.0
            );
            nuon::label()
                .y(52.0)
                .size(body_w, 22.0)
                .font_size(theme::FONT_S)
                .color(theme::text_muted())
                .text_justify(TextAlign::Start)
                .text(sub)
                .build(ui);
        });

        // ── Números da gravação ──────────────────────────────────────
        let cards_y = 92.0;
        let gap = theme::SPACE_M;
        let card_w = (body_w - gap * 3.0) / 4.0;
        let cards = [
            (
                format!("{:.0} %", take.accuracy * 100.0),
                locals::results::accuracy(),
                theme::accent(),
            ),
            (
                take.info
                    .tempo_ratio
                    .map(|r| format!("{:.0} %", r * 100.0))
                    .unwrap_or_else(|| "—".into()),
                locals::song::stat_tempo(),
                theme::text(),
            ),
            (
                format!("{:.0} ms", take.info.spread_ms),
                locals::song::stat_spread(),
                theme::text(),
            ),
            (
                take.info.hesitations.to_string(),
                locals::song::hesitations_label(),
                theme::text(),
            ),
        ];
        for (i, (value, label, color)) in cards.into_iter().enumerate() {
            nuon::translate()
                .x(x0 + i as f32 * (card_w + gap))
                .y(cards_y)
                .build(ui, |ui| {
                    theme::panel(ui, card_w, 84.0);
                    nuon::label()
                        .x(theme::SPACE_L)
                        .y(theme::SPACE_S + 2.0)
                        .size(card_w - theme::SPACE_L * 2.0, 32.0)
                        .font_size(theme::FONT_XL)
                        .bold(true)
                        .color(color)
                        .text_justify(TextAlign::Start)
                        .text(value)
                        .build(ui);
                    nuon::label()
                        .x(theme::SPACE_L)
                        .y(50.0)
                        .size(card_w - theme::SPACE_L * 2.0, 20.0)
                        .font_size(theme::FONT_S)
                        .color(theme::text_muted())
                        .text_justify(TextAlign::Start)
                        .text(label)
                        .build(ui);
                });
        }

        // ── Layout das duas colunas (dentro de um scroll) ────────────
        let insights_w = 300.0_f32.min(body_w * 0.28);
        let main_w = body_w - insights_w - theme::SPACE_XL;
        let cols_y = cards_y + 84.0 + theme::SPACE_XL;
        let cols_h = (win_h - cols_y - theme::BUTTON_H - theme::SPACE_XL * 2.0).max(160.0);

        nuon::translate()
            .x(x0)
            .y(cols_y)
            .build(ui, |ui| {
        nuon::scroll("compare_scroll")
            .scissor_size(body_w, cols_h)
            .build(ui, |ui| {
                nuon::translate()
                    .y(theme::SPACE_S)
                    .build(ui, |ui| {
            widgets::eyebrow(ui, main_w, &locals::compare::tempo_curve());
            nuon::translate().y(24.0).add_to_current(ui);

            let chart_h = 90.0;
            self::tempo_curve(ui, "cmp_tempo", main_w, chart_h, &analysis);
            nuon::translate().y(chart_h + theme::SPACE_XL).add_to_current(ui);

            // ── Mapa de compassos ────────────────────────────────────
            widgets::eyebrow(ui, main_w, &locals::compare::bars_map());
            nuon::translate().y(24.0).add_to_current(ui);

            let zoom_start = page_state(ui).zoom_start;
            let (selected, map_rows) =
                self::bars_map(ui, "cmp_map", main_w, &analysis, zoom_start);
            if let Some(i) = selected {
                page_state(ui).zoom_start = i;
            }
            // O mapa desenha com posição absoluta; avançar pelas filas
            let map_h = map_rows as f32 * 23.0;
            nuon::translate().y(map_h).add_to_current(ui);
            nuon::translate().y(theme::SPACE_XL).add_to_current(ui);

            // ── Piano roll ───────────────────────────────────────────
            widgets::eyebrow(ui, main_w, &locals::compare::piano_roll());
            nuon::translate().y(24.0).add_to_current(ui);
            self::piano_roll(ui, "cmp_roll", main_w, 190.0, &analysis, zoom_start);
            });

            // ── Coluna "O que melhorar" (à direita, no mesmo scroll) ────
            nuon::translate()
                .x(main_w + theme::SPACE_XL)
                .build(ui, |ui| {
            let _insights_col = ();

                    widgets::eyebrow(ui, insights_w, &locals::compare::improve());
                    nuon::translate().y(24.0).add_to_current(ui);

                    if analysis.insights.is_empty() {
                        widgets::hint(ui, insights_w, 60.0, &locals::compare::no_insights());
                    }

                    for (i, ins) in analysis.insights.iter().enumerate() {
                        let positive = ins.is_positive();
                        let h = 64.0;
                        nuon::translate().build(ui, |ui| {
                            let ev = nuon::click_area(("cmp_insight", i))
                                .size(insights_w, h)
                                .build(ui);
                            nuon::quad()
                                .size(insights_w, h)
                                .color(if ev.is_hovered() {
                                    theme::surface_hover()
                                } else {
                                    theme::surface_sunken()
                                })
                                .border_color(if positive {
                                    theme::good()
                                } else {
                                    theme::warn()
                                })
                                .border_width(1.0)
                                .border_radius([theme::RADIUS_M; 4])
                                .build(ui);
                            nuon::label()
                                .x(theme::SPACE_M)
                                .y(theme::SPACE_S)
                                .size(insights_w - theme::SPACE_M * 2.0, 20.0)
                                .font_size(theme::FONT_S)
                                .bold(true)
                                .color(if positive {
                                    theme::good()
                                } else {
                                    theme::warn()
                                })
                                .text_justify(TextAlign::Start)
                                .text(insight_title(&ins.kind))
                                .build(ui);
                            nuon::label()
                                .x(theme::SPACE_M)
                                .y(26.0)
                                .size(insights_w - theme::SPACE_M * 2.0, 34.0)
                                .font_size(theme::FONT_S - 1.0)
                                .color(theme::text_muted())
                                .text_justify(TextAlign::Start)
                                .text(theme::fit_text(
                                    &ins.text,
                                    insights_w - theme::SPACE_M * 2.0,
                                    theme::FONT_S - 1.0,
                                ))
                                .build(ui);
                            if ev.is_clicked() {
                                let ps = page_state(ui);
                                ps.zoom_start = ins.bars.0.saturating_sub(1);
                            }
                        });
                        nuon::translate().y(h + theme::SPACE_S).add_to_current(ui);
                    }
                });
            });
        });

        // ── Barra inferior ───────────────────────────────────────────
        let bar_h = theme::BUTTON_H + 8.0;
        nuon::translate()
            .x(x0)
            .y(win_h - theme::SPACE_L - bar_h)
            .build(ui, |ui| {
                if widgets::button(("cmp_back", 0), "")
                    .icon(icons::left_arrow_icon())
                    .size(bar_h, bar_h)
                    .build(ui)
                    .clicked
                {
                    act = Some(Act::Back);
                }
                nuon::translate().x(bar_h + theme::SPACE_M).add_to_current(ui);
                if widgets::button(("cmp_view", 0), locals::results::view_recording())
                    .icon(icons::play_icon())
                    .size(190.0, bar_h)
                    .build(ui)
                    .clicked
                {
                    act = Some(Act::ViewTake);
                }
            });

        self.apply_nav_actions(ctx, nav_act);

        let Some(act) = act else { return };
        match act {
            Act::Back => self.state.go_back(),
            Act::ViewTake => {
                use midi_file::MidiFile;
                if let Ok(midi) = MidiFile::new(&take.midi_path) {
                    super::state::connect_io(&self.state, ctx);
                    ctx.proxy
                        .send_event(crate::NeothesiaEvent::Play(crate::song::Song::new(midi)))
                        .ok();
                }
            }
        }
        let _ = PathBuf::new();
    }
}

/// Título curto do tipo de sugestão
fn insight_title(kind: &InsightKind) -> &'static str {
    match kind {
        InsightKind::ChordSpread => locals::compare::ins_chords(),
        InsightKind::SlowDown => locals::compare::ins_slow(),
        InsightKind::Hesitation => locals::compare::ins_hesitation(),
        InsightKind::Strength => locals::compare::ins_strength(),
    }
}

/// Curva de andamento por compasso, com a linha típica a tracejado e pontos
/// laranja nos compassos com hesitações.
fn tempo_curve(ui: &mut nuon::Ui, id: &str, w: f32, h: f32, analysis: &Analysis) {
    let tempos: Vec<f32> = analysis
        .bars
        .iter()
        .map(|b| b.tempo.unwrap_or(analysis.tempo))
        .collect();
    if tempos.len() < 2 {
        return;
    }
    let lo = tempos.iter().cloned().fold(f32::MAX, f32::min);
    let hi = tempos.iter().cloned().fold(f32::MIN, f32::max);
    let (lo, hi) = {
        let pad = ((hi - lo) * 0.15).max(0.05);
        ((lo - pad).max(0.0), hi + pad)
    };
    let n = tempos.len();
    let dx = w / (n - 1) as f32;
    let y_of = |v: f32| h - (v - lo) / (hi - lo).max(1e-3) * h;

    // Linha da curva (segmentos)
    for i in 0..n - 1 {
        let x0 = i as f32 * dx;
        let x1 = (i + 1) as f32 * dx;
        let y0 = y_of(tempos[i]);
        let y1 = y_of(tempos[i + 1]);
        let steps = ((x1 - x0) / 6.0).ceil().max(1.0) as usize;
        for s in 0..steps {
            let t0 = s as f32 / steps as f32;
            let t1 = (s + 1) as f32 / steps as f32;
            let xa = x0 + (x1 - x0) * t0;
            let xb = x0 + (x1 - x0) * t1;
            let ya = y0 + (y1 - y0) * t0;
            let yb = y0 + (y1 - y0) * t1;
            let (top, height) = if ya <= yb {
                (ya, (yb - ya).max(1.5))
            } else {
                (yb, (ya - yb).max(1.5))
            };
            nuon::quad()
                .pos(xa, top)
                .size((xb - xa).max(1.0), height)
                .color(theme::accent())
                .build(ui);
        }
    }

    // Andamento típico, a tracejado
    let y = y_of(analysis.tempo);
    let dash: f32 = 8.0;
    let mut x = 0.0;
    while x < w {
        nuon::quad()
            .pos(x, y - 1.0)
            .size(dash.min(w - x), 1.5)
            .color(theme::text_faint())
            .build(ui);
        x += dash * 2.0;
    }

    // Hesitações: pontos laranja
    for (i, b) in analysis.bars.iter().enumerate() {
        if b.hesitations > 0 {
            nuon::circle(4.0)
                .pos(i as f32 * dx - 4.0, y_of(tempos[i]) - 4.0)
                .color(theme::warn())
                .build(ui);
        }
    }

    // Rótulos dos eixos
    let fmt = |v: f32| format!("{:.0} %", v * 100.0);
    nuon::label()
        .size(80.0, 16.0)
        .font_size(theme::FONT_S - 1.0)
        .color(theme::text_faint())
        .text_justify(TextAlign::Start)
        .text(fmt(hi))
        .build(ui);
    nuon::label()
        .y(h - 16.0)
        .size(80.0, 16.0)
        .font_size(theme::FONT_S - 1.0)
        .color(theme::text_faint())
        .text_justify(TextAlign::Start)
        .text(fmt(lo))
        .build(ui);
    let _ = id;
}

/// Mapa de compassos: um quadrado por compasso, cor pela qualidade. Devolve o
/// compasso clicado (para o zoom).
/// Devolve `(compasso clicado, filas usadas)` — o chamador avança o cursor
/// pelas filas, porque os quadrados são desenhados com posição absoluta.
fn bars_map(
    ui: &mut nuon::Ui,
    id: &str,
    w: f32,
    analysis: &Analysis,
    zoom_start: usize,
) -> (Option<usize>, usize) {
    let n = analysis.bars.len();
    if n == 0 {
        return (None, 0);
    }
    let sq = 20.0;
    let gap = 3.0;
    let per_row = ((w + gap) / (sq + gap)).floor().max(1.0) as usize;
    let mut clicked = None;

    for (i, b) in analysis.bars.iter().enumerate() {
        let col = i % per_row;
        let row = i / per_row;
        let x = col as f32 * (sq + gap);
        let y = row as f32 * (sq + gap);

        let color = match b.score {
            Some(s) if s >= 0.85 => theme::good(),
            Some(s) if s >= 0.6 => theme::warn().mix(theme::surface_sunken(), 0.6),
            Some(_) => theme::warn(),
            None => theme::border_strong(),
        };

        let ev = nuon::click_area((id, i)).pos(x, y).size(sq, sq).build(ui);
        let is_zoom = i >= zoom_start && i < zoom_start + ZOOM_BARS;
        nuon::quad()
            .pos(x, y)
            .size(sq, sq)
            .color(if ev.is_hovered() {
                color.mix(theme::text(), 0.2)
            } else {
                color
            })
            .border_color(if is_zoom {
                theme::text()
            } else {
                nuon::Color::new_u8(0, 0, 0, 0.0)
            })
            .border_width(if is_zoom { 1.5 } else { 0.0 })
            .border_radius([3.0; 4])
            .build(ui);

        if ev.is_clicked() {
            clicked = Some(i.min(n.saturating_sub(ZOOM_BARS)));
        }
    }
    let rows = n.div_ceil(per_row);
    (clicked, rows)
}

/// Piano roll de `ZOOM_BARS` compassos: esperadas em contorno marfim,
/// tocadas a cheio (good/warn), acordes espalhados assinalados.
fn piano_roll(
    ui: &mut nuon::Ui,
    id: &str,
    w: f32,
    h: f32,
    analysis: &Analysis,
    zoom_start: usize,
) {
    let n = analysis.bars.len();
    if n == 0 {
        return;
    }
    let b0 = zoom_start.min(n.saturating_sub(1));
    let b1 = (b0 + ZOOM_BARS).min(n);
    let t0 = analysis.bars[b0].start;
    let t1 = analysis.bars[b1 - 1].end.max(t0 + 0.5);

    // Gama de notas visíveis
    let mut lo = u8::MAX;
    let mut hi = 0_u8;
    for m in &analysis.notes {
        if m.start >= t0 && m.start < t1 {
            lo = lo.min(m.note);
            hi = hi.max(m.note);
        }
    }
    for p in &analysis.played {
        if p.start >= t0 && p.start < t1 {
            lo = lo.min(p.note);
            hi = hi.max(p.note);
        }
    }
    if lo > hi {
        lo = 48;
        hi = 72;
    }
    lo = lo.saturating_sub(2);
    hi = hi.saturating_add(2);
    let pitch_h = h / (hi - lo + 1) as f32;
    let y_of = |note: u8| h - (note - lo + 1) as f32 * pitch_h;
    let x_of = |t: f64| ((t - t0) / (t1 - t0)).clamp(0.0, 1.0) as f32 * w;

    // Fundo
    nuon::quad()
        .size(w, h)
        .color(theme::surface_sunken())
        .border_color(theme::border())
        .border_width(1.0)
        .border_radius([theme::RADIUS_M; 4])
        .build(ui);
    nuon::layer().build(ui, |ui| {
        // Linhas de separação dos compassos
        for b in &analysis.bars[b0..b1] {
            let x = x_of(b.start);
            nuon::quad()
                .pos(x, 0.0)
                .size(1.0, h)
                .color(theme::border())
                .build(ui);
        }

        // Notas esperadas: contorno marfim
        for m in &analysis.notes {
            if m.start < t0 || m.start >= t1 {
                continue;
            }
            let x = x_of(m.start);
            let xe = x_of(m.end).max(x + 3.0);
            nuon::quad()
                .pos(x, y_of(m.note))
                .size(xe - x, pitch_h - 1.0)
                .color(nuon::Color::new_u8(0, 0, 0, 0.0))
                .border_color(theme::text().mix(theme::surface_sunken(), 0.4))
                .border_width(1.0)
                .border_radius([2.0; 4])
                .build(ui);
        }

        // Notas tocadas: cheio
        for p in &analysis.played {
            if p.start < t0 || p.start >= t1 {
                continue;
            }
            let x = x_of(p.start);
            let xe = x_of(p.end).max(x + 3.0);
            nuon::quad()
                .pos(x, y_of(p.note))
                .size(xe - x, pitch_h - 1.0)
                .color(if p.off_time { theme::warn() } else { theme::good() })
                .border_radius([2.0; 4])
                .build(ui);
        }

        // Acordes espalhados: retângulo tracejado laranja sobre o compasso
        for b in &analysis.bars[b0..b1] {
            if b.chord_spread_ms < CHORD_WARN_MS {
                continue;
            }
            let x = x_of(b.start);
            let xe = x_of(b.end);
            dashed_rect(ui, x + 1.0, 1.0, xe - x - 2.0, h - 2.0, theme::warn());
        }
    });
    let _ = id;
}

/// Retângulo tracejado (4 lados em segmentos)
fn dashed_rect(ui: &mut nuon::Ui, x: f32, y: f32, w: f32, h: f32, color: nuon::Color) {
    let dash: f32 = 6.0;
    let mut px = x;
    while px < x + w {
        let d = dash.min(x + w - px);
        nuon::quad().pos(px, y).size(d, 1.5).color(color).build(ui);
        nuon::quad()
            .pos(px, y + h - 1.5)
            .size(d, 1.5)
            .color(color)
            .build(ui);
        px += dash * 2.0;
    }
    let mut py = y;
    while py < y + h {
        let d = dash.min(y + h - py);
        nuon::quad().pos(x, py).size(1.5, d).color(color).build(ui);
        nuon::quad()
            .pos(x + w - 1.5, py)
            .size(1.5, d)
            .color(color)
            .build(ui);
        py += dash * 2.0;
    }
}
