//! Página "Resultado" (R5): precisão grande em serifa, cartões de números,
//! ritmo, gráfico de erros por compasso com good()/warn() e ações.
//!
//! Mostrada ao voltar do ecrã de tocar quando houve gravação
//! (`self.state.result`, vindo de `ctx.last_result`).

use nuon::TextAlign;

use super::library_page::play_path;
use crate::{
    context::Context,
    icons,
    locals,
    recording::Report,
    theme,
    widgets,
};

enum Act {
    Back,
    ViewTake,
    Recordings,
    Replay,
    Next,
}

/// Etiqueta curta do ritmo, a partir dos mesmos limites do `Report`
fn timing_label(report: &Report) -> String {
    if report.hits == 0 {
        return locals::results::timing_none();
    }
    let m = report.mean_offset_ms;
    if m.abs() < 15.0 {
        locals::results::timing_on_time()
    } else if m.abs() < 40.0 {
        if m > 0.0 {
            locals::results::timing_slightly_late()
        } else {
            locals::results::timing_slightly_early()
        }
    } else if m > 0.0 {
        locals::results::timing_late()
    } else {
        locals::results::timing_early()
    }
}

/// Tooltip desenhado por cima da âncora (posição atual do cursor de desenho)
fn tooltip_above(ui: &mut nuon::Ui, anchor_x: f32, text: &str) {
    let w = text.chars().count() as f32 * theme::FONT_S * 0.8 + 16.0;
    let h = 26.0;
    nuon::layer().overlay(true).build(ui, |ui| {
        nuon::translate()
            .x(anchor_x - w / 2.0)
            .y(-h - 8.0)
            .build(ui, |ui| {
                nuon::quad()
                    .size(w, h)
                    .color(theme::surface())
                    .border_color(theme::border_strong())
                    .border_width(1.0)
                    .border_radius([theme::RADIUS_S; 4])
                    .build(ui);
                nuon::label()
                    .size(w, h)
                    .font_size(theme::FONT_S)
                    .color(theme::text())
                    .text(text)
                    .build(ui);
            });
    });
}

/// Cartão de número (estilo R3): valor grande + legenda pequena
fn stat_card(ui: &mut nuon::Ui, w: f32, h: f32, value: &str, label: &str, value_color: nuon::Color) {
    theme::panel(ui, w, h);
    nuon::label()
        .x(theme::SPACE_L)
        .y(theme::SPACE_M)
        .size(w - theme::SPACE_L * 2.0, 34.0)
        .font_size(theme::FONT_XL)
        .bold(true)
        .color(value_color)
        .text_justify(TextAlign::Start)
        .text(value)
        .build(ui);
    nuon::label()
        .x(theme::SPACE_L)
        .y(52.0)
        .size(w - theme::SPACE_L * 2.0, 20.0)
        .font_size(theme::FONT_S)
        .color(theme::text_muted())
        .text_justify(TextAlign::Start)
        .text(label)
        .build(ui);
}

impl super::MenuScene {
    pub fn results_page_window_event(
        &mut self,
        ctx: &mut Context,
        event: &winit::event::WindowEvent,
    ) {
        use crate::utils::window::WinitEvent;
        use winit::keyboard::{Key, NamedKey};
        if event.key_pressed(Key::Named(NamedKey::Escape)) {
            self.state.go_back();
        } else if event.key_pressed(Key::Named(NamedKey::Enter)) {
            super::state::play(&self.state, ctx);
        }
    }

    pub fn results_page_ui(&mut self, ctx: &mut Context, ui: &mut nuon::Ui) {
        let win_h = ctx.window_state.logical_size.height;

        let (nav, nav_act) = self.side_nav(ctx, ui);
        let mut act: Option<Act> = None;

        let Some(result) = self.state.result.clone() else {
            self.state.go_back();
            return;
        };
        let (take, report) = (&result.take, &result.report);

        let body_w = nav.content_w - theme::SPACE_XL * 2.0;
        let x0 = nav.content_x + theme::SPACE_XL;

        // ── Cabeçalho: música + data · duração ───────────────────────
        let (title, composer) = crate::library::Library::pretty_title(&take.song);
        nuon::translate().x(x0).y(theme::SPACE_S).build(ui, |ui| {
            widgets::page_title(ui, body_w, &title);
            let right = format!(
                "{}{} · {}:{:02}",
                composer.map(|c| format!("{c} · ")).unwrap_or_default(),
                take.date_label(),
                take.duration.as_secs() / 60,
                take.duration.as_secs() % 60
            );
            nuon::label()
                .y(52.0)
                .size(body_w, 22.0)
                .font_size(theme::FONT_S)
                .color(theme::text_muted())
                .text_justify(TextAlign::Start)
                .text(right)
                .build(ui);
        });

        // ── Precisão (serifa grande) + cartões de números ────────────
        let hero_y = 84.0;
        let hero_h = 132.0;
        let hero_w = if body_w >= 620.0 { body_w * 0.35 } else { body_w };
        let cards_w = body_w - hero_w - theme::SPACE_M;

        nuon::translate().x(x0).y(hero_y).build(ui, |ui| {
            theme::panel(ui, hero_w, hero_h);
            let acc = (report.accuracy * 100.0).round();
            nuon::label()
                .x(theme::SPACE_L)
                .y(theme::SPACE_M)
                .size(hero_w - theme::SPACE_L * 2.0, 72.0)
                .font_size(56.0)
                .font_family(theme::FONT_DISPLAY)
                .bold(true)
                .color(theme::accent())
                .text_justify(TextAlign::Start)
                .text(format!("{acc:.0} %"))
                .build(ui);
            nuon::label()
                .x(theme::SPACE_L)
                .y(96.0)
                .size(hero_w - theme::SPACE_L * 2.0, 22.0)
                .font_size(theme::FONT_S)
                .color(theme::text_muted())
                .text_justify(TextAlign::Start)
                .text(locals::results::accuracy())
                .build(ui);
        });

        // Comparação com a gravação anterior ("+6 % do que a última vez")
        let takes = ctx.takes.list(&take.song);
        let comparison = {
            let pos = takes.iter().position(|t| t.recorded_at == take.recorded_at);
            match pos.and_then(|p| takes.get(p + 1)) {
                Some(prev) => {
                    let delta = (report.accuracy - prev.accuracy) * 100.0;
                    if delta.abs() < 0.5 {
                        locals::results::same_as_previous()
                    } else {
                        locals::results::vs_previous(delta)
                    }
                }
                None => String::new(),
            }
        };

        let cards_y = hero_y;
        let card_w = (cards_w - theme::SPACE_M * 2.0) / 3.0;
        let cards: [(String, usize, nuon::Color); 3] = [
            (locals::results::hits(), report.hits, theme::good()),
            (locals::results::missed(), report.missed, theme::warn()),
            (locals::results::extra(), report.extra, theme::text_muted()),
        ];
        for (i, (label, n, color)) in cards.into_iter().enumerate() {
            nuon::translate()
                .x(x0 + hero_w + theme::SPACE_M + i as f32 * (card_w + theme::SPACE_M))
                .y(cards_y)
                .build(ui, |ui| {
                    stat_card(ui, card_w, hero_h, &n.to_string(), &label, color);
                });
        }

        // Ritmo + comparação
        let rhythm_y = hero_y + hero_h + theme::SPACE_S;
        nuon::translate().x(x0).y(rhythm_y).build(ui, |ui| {
            nuon::label()
                .size(body_w, 22.0)
                .font_size(theme::FONT_S)
                .color(theme::text_muted())
                .text_justify(TextAlign::Start)
                .text(locals::results::rhythm(
                    report.mean_offset_ms,
                    &timing_label(report),
                    report.offset_spread_ms,
                ))
                .build(ui);
            if !comparison.is_empty() {
                nuon::label()
                    .y(24.0)
                    .size(body_w, 22.0)
                    .font_size(theme::FONT_S)
                    .color(theme::good())
                    .text_justify(TextAlign::Start)
                    .text(&comparison)
                    .build(ui);
            }
        });

        // ── Erros por compasso (good/warn) ───────────────────────────
        let chart_y = rhythm_y + (if comparison.is_empty() { 22.0 } else { 46.0 })
            + theme::SPACE_L;
        nuon::translate().x(x0).y(chart_y).build(ui, |ui| {
            widgets::eyebrow(ui, body_w, &locals::results::sections_title());
            nuon::translate().y(24.0).add_to_current(ui);

            let errors: Vec<f32> = report
                .sections
                .iter()
                .map(|s| (s.missed + s.extra) as f32)
                .collect();
            let n = errors.len().max(1);
            let chart_h = (win_h - chart_y - 24.0 - theme::BUTTON_H - theme::SPACE_XL * 2.5)
                .clamp(80.0, 200.0);
            let max = errors.iter().cloned().fold(1.0_f32, f32::max);

            let bar_gap = 3.0;
            let bar_w = ((body_w - bar_gap * (n - 1) as f32) / n as f32).max(2.0);
            let mut hovered = None;
            for (i, &v) in errors.iter().enumerate() {
                let x = i as f32 * (bar_w + bar_gap);
                let bh = if max > 0.0 { v / max * chart_h } else { 0.0 };
                let ev = nuon::click_area(("res_bar", i))
                    .pos(x, chart_h - bh.max(8.0))
                    .size(bar_w, bh.max(8.0))
                    .build(ui);
                // Compasso limpo: marca curta em good(); com erros: warn()
                let (bh, color) = if v == 0.0 {
                    (4.0, theme::good())
                } else {
                    (v / max * chart_h, theme::warn())
                };
                nuon::quad()
                    .pos(x + bar_gap / 2.0, chart_h - bh)
                    .size((bar_w - bar_gap).max(1.0), bh)
                    .color(color.mix(theme::surface_sunken(), 0.25))
                    .border_radius([2.0; 4])
                    .build(ui);
                if ev.is_hovered() {
                    hovered = Some(i);
                }
            }

            // Base do gráfico
            nuon::quad()
                .y(chart_h)
                .size(body_w, 1.0)
                .color(theme::border())
                .build(ui);

            if let Some(i) = hovered
                && let Some(s) = report.sections.get(i)
            {
                tooltip_above(
                    ui,
                    i as f32 * (bar_w + bar_gap) + bar_w / 2.0,
                    &locals::results::section_tooltip(i + 1, s.missed, s.extra),
                );
            }
        });

        // ── Barra inferior ───────────────────────────────────────────
        let bar_h = theme::BUTTON_H + 8.0;
        let has_next = ctx.library.has_next();
        nuon::translate().x(x0).y(win_h - theme::SPACE_L - bar_h).build(ui, |ui| {
            if widgets::button(("res_back", 0), "")
                .icon(icons::left_arrow_icon())
                .size(bar_h, bar_h)
                .build(ui)
                .clicked
            {
                act = Some(Act::Back);
            }

            if body_w >= 620.0 {
                // Uma linha: Ver gravação · Gravações · Repetir · Seguinte
                let mut x = body_w;
                if has_next {
                    x -= 170.0;
                    nuon::translate().x(x).build(ui, |ui| {
                        if widgets::button(("res_next", 0), locals::results::next())
                            .icon(icons::play_icon())
                            .primary()
                            .size(170.0, bar_h)
                            .build(ui)
                            .clicked
                        {
                            act = Some(Act::Next);
                        }
                    });
                    x -= theme::SPACE_M;
                }
                x -= 150.0;
                nuon::translate().x(x).build(ui, |ui| {
                    let b = widgets::button(("res_replay", 0), locals::results::replay())
                        .size(150.0, bar_h);
                    let b = if has_next { b } else { b.primary() };
                    if b.build(ui).clicked {
                        act = Some(Act::Replay);
                    }
                });
                x -= theme::SPACE_M + 160.0;
                nuon::translate().x(x).build(ui, |ui| {
                    if widgets::button(("res_recs", 0), locals::results::recordings())
                        .size(160.0, bar_h)
                        .build(ui)
                        .clicked
                    {
                        act = Some(Act::Recordings);
                    }
                });
                x -= theme::SPACE_M + 150.0;
                nuon::translate().x(x).build(ui, |ui| {
                    if widgets::button(("res_view", 0), locals::results::view_recording())
                        .size(150.0, bar_h)
                        .build(ui)
                        .clicked
                    {
                        act = Some(Act::ViewTake);
                    }
                });
            } else {
                // Duas linhas a 670 px: navegação em cima, ações em baixo
                let mut x = body_w;
                x -= 150.0;
                nuon::translate().x(x).build(ui, |ui| {
                    if widgets::button(("res_view", 0), locals::results::view_recording())
                        .size(150.0, bar_h)
                        .build(ui)
                        .clicked
                    {
                        act = Some(Act::ViewTake);
                    }
                });
                x -= theme::SPACE_M + 130.0;
                nuon::translate().x(x).build(ui, |ui| {
                    if widgets::button(("res_recs", 0), locals::results::recordings())
                        .size(130.0, bar_h)
                        .build(ui)
                        .clicked
                    {
                        act = Some(Act::Recordings);
                    }
                });

                nuon::translate().y(bar_h + theme::SPACE_S).build(ui, |ui| {
                    let mut x = body_w;
                    if has_next {
                        x -= 150.0;
                        nuon::translate().x(x).build(ui, |ui| {
                            if widgets::button(("res_next", 0), locals::results::next())
                                .icon(icons::play_icon())
                                .primary()
                                .size(150.0, bar_h)
                                .build(ui)
                                .clicked
                            {
                                act = Some(Act::Next);
                            }
                        });
                        x -= theme::SPACE_M;
                    }
                    x -= 130.0;
                    nuon::translate().x(x).build(ui, |ui| {
                        let b = widgets::button(("res_replay", 0), locals::results::replay())
                            .size(130.0, bar_h);
                        let b = if has_next { b } else { b.primary() };
                        if b.build(ui).clicked {
                            act = Some(Act::Replay);
                        }
                    });
                });
            }
        });

        self.apply_nav_actions(ctx, nav_act);

        // ── Aplicar ação ─────────────────────────────────────────────
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
            Act::Recordings => self.state.open_recordings(ctx, take.song.clone()),
            Act::Replay => super::state::play(&self.state, ctx),
            Act::Next => {
                if let Some(path) = ctx.library.advance() {
                    let pos = ctx.library.queue_pos;
                    play_path(&self.state, ctx, path, pos);
                }
            }
        }
    }
}
