//! Página de uma música (R3, maqueta "Página da música"): migalhas, título em
//! serifa, ações, números da melhor gravação, evolução por métrica e lista de
//! gravações com a barra do trecho tocado.
//!
//! Dados: `self.state.song_page` (path) e `self.state.recordings` (mais
//! recente primeiro, carregados por `UiState::open_song`).

use std::{hash::Hash, path::PathBuf};

use midi_file::MidiFile;
use nuon::TextAlign;

use super::library_page::{is_partial, play_path, segmented_labels};
use crate::{context::Context, icons, library::Library, locals, recording::Take, theme, widgets};

const TITLE_SIZE: f32 = 46.0;

enum Act {
    Play,
    OpenTake(usize),
    DeleteTake(usize),
}

/// Estado local da página
#[derive(Default)]
struct PageState {
    metric: usize,
    /// Índice da gravação com apagar pendente (confirmação em 3 s)
    pending_delete: Option<usize>,
    pending_delete_at: Option<std::time::Instant>,
}

fn page_state(ui: &mut nuon::Ui) -> &mut PageState {
    ui.data::<PageState>(widgets::wid(("song", "page_state")))
}

/// Duração da música (max end das notas), calculada uma única vez por path
pub(super) fn song_duration(ctx: &Context, ui: &mut nuon::Ui, path: &PathBuf) -> f32 {
    let cache: &mut Option<(PathBuf, f32)> = ui.data(widgets::wid(("song", "dur_cache")));
    if let Some((p, d)) = cache.as_ref()
        && p == path
    {
        return *d;
    }
    let dur = MidiFile::new(path)
        .map(|f| {
            f.tracks
                .iter()
                .flat_map(|t| t.notes.iter())
                .map(|n| n.end)
                .max()
                .unwrap_or_default()
                .as_secs_f32()
        })
        .unwrap_or(0.0);
    *cache = Some((path.clone(), dur));
    dur
}

/// Uma gravação é "completa" se cobrir ≥ 90 % da duração da música
/// (gravações automáticas, sem `covered`, são sempre completas).
pub(super) fn is_complete(t: &Take, song_dur: f32) -> bool {
    match t.info.covered {
        None => true,
        Some((a, b)) => {
            song_dur <= 0.0 || (b.saturating_sub(a)).as_secs_f32() / song_dur >= 0.9
        }
    }
}

/// Melhor gravação completa (ou, em falta, a melhor de todas)
fn best_take(takes: &[Take], song_dur: f32) -> Option<&Take> {
    let complete: Vec<&Take> = takes.iter().filter(|t| is_complete(t, song_dur)).collect();
    complete
        .iter()
        .max_by(|a, b| a.accuracy.total_cmp(&b.accuracy))
        .copied()
        .or_else(|| takes.iter().max_by(|a, b| a.accuracy.total_cmp(&b.accuracy)))
}

impl super::MenuScene {
    pub fn song_page_window_event(
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

    pub fn song_page_ui(&mut self, ctx: &mut Context, ui: &mut nuon::Ui) {
        let win_h = ctx.window_state.logical_size.height;

        let (nav, nav_act) = self.side_nav(ctx, ui);
        let mut act: Option<Act> = None;

        let Some(song) = self.state.song_page.clone() else {
            self.state.go_back();
            return;
        };
        let takes = self.state.recordings.clone();
        let dur = song_duration(ctx, ui, &song);
        let best = best_take(&takes, dur).cloned();
        let first = takes.last().cloned();

        let body_w = nav.content_w - theme::SPACE_XL * 2.0;
        let x0 = nav.content_x + theme::SPACE_XL;

        // ── Migalhas: Biblioteca › Autor ─────────────────────────────
        nuon::translate().x(x0).y(theme::SPACE_L).build(ui, |ui| {
            if widgets::button(("song_crumb", 0), locals::library::title())
                .ghost()
                .font_size(theme::FONT_S)
                .size(120.0, 28.0)
                .build(ui)
                .clicked
            {
                self.state.go_back();
            }
            nuon::label()
                .x(124.0)
                .size(20.0, 28.0)
                .font_size(theme::FONT_S)
                .color(theme::text_faint())
                .text("›")
                .build(ui);
            // Nome canónico do autor (o mesmo que a Biblioteca mostra)
            let composer = Library::pretty_title(&song)
                .1
                .and_then(|c| {
                    ctx.library
                        .by_composer()
                        .into_iter()
                        .find(|g| {
                            g.name.to_lowercase().contains(&c.to_lowercase())
                                || c.to_lowercase().contains(&g.name.to_lowercase())
                        })
                        .map(|g| g.name)
                })
                .unwrap_or_else(|| Library::pretty_title(&song).1.unwrap_or_default());
            nuon::label()
                .x(134.0)
                .size(body_w - 134.0, 28.0)
                .font_size(theme::FONT_S)
                .color(theme::accent())
                .text_justify(TextAlign::Start)
                .text(composer)
                .build(ui);
        });

        // ── Título em serifa 46 px ───────────────────────────────────
        let (title, _) = Library::pretty_title(&song);
        nuon::translate()
            .x(x0)
            .y(theme::SPACE_L + 34.0)
            .build(ui, |ui| {
                nuon::label()
                    .size(body_w, 56.0)
                    .font_size(TITLE_SIZE)
                    .font_family(theme::FONT_DISPLAY)
                    .color(theme::text())
                    .text_justify(TextAlign::Start)
                    .text(theme::fit_text(&title, body_w, TITLE_SIZE))
                    .build(ui);
            });

        // ── Ações ────────────────────────────────────────────────────
        let actions_y = theme::SPACE_L + 34.0 + 56.0 + theme::SPACE_M;
        nuon::translate().x(x0).y(actions_y).build(ui, |ui| {
            if widgets::button(("song_play", 0), locals::song::play())
                .icon(icons::play_icon())
                .primary()
                .size(150.0, theme::BUTTON_H)
                .build(ui)
                .clicked
            {
                act = Some(Act::Play);
            }
            nuon::translate().x(150.0 + theme::SPACE_M).add_to_current(ui);

            // A comparação visual chega com a R6; desativado até lá
            // Secondary é o estilo por omissão do botão
            widgets::button(("song_compare", 0), locals::song::compare_best())
                .size(190.0, theme::BUTTON_H)
                .build(ui);
            nuon::translate().x(190.0 + theme::SPACE_M).add_to_current(ui);

            // A edição chega com a B10; desativado até lá
            widgets::button(("song_edit", 0), locals::song::edit())
                .size(120.0, theme::BUTTON_H)
                .build(ui);
        });

        // ── Números da melhor gravação completa ──────────────────────
        let cards_y = actions_y + theme::BUTTON_H + theme::SPACE_XL;
        nuon::translate().x(x0).y(cards_y).build(ui, |ui| {
            let gap = theme::SPACE_M;
            let card_w = (body_w - gap * 3.0) / 4.0;
            let card_h = 96.0;

            let first_is_best = first.as_ref().is_some() && first == best;
            let era_tempo = first
                .as_ref()
                .filter(|_| !first_is_best)
                .and_then(|f| f.info.tempo_ratio)
                .map(|r| locals::song::was_before(&format!("{:.0} %", r * 100.0)));
            let era_spread = first
                .as_ref()
                .filter(|_| !first_is_best)
                .map(|f| locals::song::was_before(&format!("{:.0} ms", f.info.spread_ms)));
            let era_hes = first
                .as_ref()
                .filter(|_| !first_is_best)
                .map(|f| locals::song::was_before(&f.info.hesitations.to_string()));

            let stats: [(String, String, Option<String>); 4] = best
                .as_ref()
                .map(|b| {
                    let acc_pct = format!("{:.1} %", b.accuracy * 100.0).replace('.', ",");
                    [
                        (
                            acc_pct,
                            format!(
                                "{} de {}",
                                b.hits,
                                b.hits + b.missed
                            ),
                            None,
                        ),
                        (
                            b.info
                                .tempo_ratio
                                .map(|r| format!("{:.0} %", r * 100.0))
                                .unwrap_or_else(|| "—".into()),
                            locals::song::stat_tempo(),
                            era_tempo.clone(),
                        ),
                        (
                            format!("{:.0} ms", b.info.spread_ms),
                            locals::song::stat_spread(),
                            era_spread.clone(),
                        ),
                        (
                            b.info.hesitations.to_string(),
                            locals::song::hesitations_label(),
                            era_hes.clone(),
                        ),
                    ]
                })
                .unwrap_or([
                    ("—".into(), locals::song::stat_hits(), None),
                    ("—".into(), locals::song::stat_tempo(), None),
                    ("—".into(), locals::song::stat_spread(), None),
                    ("—".into(), locals::song::stat_hesitations(0), None),
                ]);

            for (i, (value, label, era)) in stats.into_iter().enumerate() {
                nuon::translate()
                    .x(i as f32 * (card_w + gap))
                    .build(ui, |ui| {
                        theme::panel(ui, card_w, card_h);
                        nuon::label()
                            .x(theme::SPACE_L)
                            .y(theme::SPACE_M)
                            .size(card_w - theme::SPACE_L * 2.0, 34.0)
                            .font_size(theme::FONT_XL)
                            .bold(true)
                            .color(theme::text())
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
                        if let Some(era) = era {
                            nuon::label()
                                .x(theme::SPACE_L)
                                .y(69.0)
                                .size(card_w - theme::SPACE_L * 2.0, 18.0)
                                .font_size(theme::FONT_S - 1.0)
                                .color(theme::text_faint())
                                .text_justify(TextAlign::Start)
                                .text(era)
                                .build(ui);
                        }
                    });
            }
        });

        // ── Evolução ─────────────────────────────────────────────────
        let evo_y = cards_y + 96.0 + theme::SPACE_XL;
        nuon::translate().x(x0).y(evo_y).build(ui, |ui| {
            widgets::section_title(ui, body_w, &locals::song::evolution(), None);
            nuon::translate().y(34.0).add_to_current(ui);

            let metrics = [
                locals::song::metric_accuracy(),
                locals::song::metric_tempo(),
                locals::song::metric_spread(),
                locals::song::metric_hesitations(),
            ];

            let metric = page_state(ui).metric.min(3);

            if let Some(i) = segmented_labels(ui, "song_metric", 420.0, &metrics, metric) {
                page_state(ui).metric = i;
            }

            nuon::translate().y(42.0).add_to_current(ui);

            let values: Vec<f32> = takes
                .iter()
                .rev()
                .map(|t| match metric {
                    0 => t.accuracy,
                    1 => t.info.tempo_ratio.unwrap_or(0.0),
                    2 => t.info.spread_ms,
                    _ => t.info.hesitations as f32,
                })
                .collect();

            if takes.len() >= 2 {
                let chart_h = 140.0;
                let fmt = |v: f32| -> String {
                    match metric {
                        0 => format!("{:.0} %", v * 100.0),
                        1 => format!("{:.0} %", v * 100.0),
                        2 => format!("{:.0} ms", v),
                        _ => format!("{:.0}", v),
                    }
                };
                let (lo, hi) = widgets::auto_range(&values);
                let hovered =
                    widgets::line_chart(ui, "song_evo", body_w, chart_h, &values);

                // Eixo esquerdo: máximo (topo) e mínimo (fundo)
                nuon::label()
                    .y(0.0)
                    .size(70.0, 18.0)
                    .font_size(theme::FONT_S - 1.0)
                    .color(theme::text_faint())
                    .text_justify(TextAlign::Start)
                    .text(fmt(hi))
                    .build(ui);
                nuon::label()
                    .y(chart_h - 18.0)
                    .size(70.0, 18.0)
                    .font_size(theme::FONT_S - 1.0)
                    .color(theme::text_faint())
                    .text_justify(TextAlign::Start)
                    .text(fmt(lo))
                    .build(ui);
                if metric >= 2 {
                    nuon::label()
                        .y(chart_h + 4.0)
                        .size(body_w, 18.0)
                        .font_size(theme::FONT_S - 1.0)
                        .color(theme::text_faint())
                        .text_justify(TextAlign::End)
                        .text(locals::song::lower_is_better())
                        .build(ui);
                }

                // Datas da primeira (mais antiga) e da última gravação
                if let (Some(oldest), Some(newest)) = (takes.last(), takes.first()) {
                    nuon::label()
                        .y(chart_h + 22.0)
                        .size(body_w, 18.0)
                        .font_size(theme::FONT_S - 1.0)
                        .color(theme::text_faint())
                        .text_justify(TextAlign::Start)
                        .text(oldest.date_label())
                        .build(ui);
                    nuon::label()
                        .y(chart_h + 22.0)
                        .size(body_w, 18.0)
                        .font_size(theme::FONT_S - 1.0)
                        .color(theme::text_faint())
                        .text_justify(TextAlign::End)
                        .text(newest.date_label())
                        .build(ui);
                }

                // Tooltip com o valor do ponto sob o rato
                if let Some(i) = hovered
                    && let Some(v) = values.get(i)
                    && let Some(t) = takes.get(takes.len() - 1 - i)
                {
                    let text = format!("{} · {}", fmt(*v), t.date_label());
                    let w = text.chars().count() as f32 * theme::FONT_S * 0.62 + 16.0;
                    nuon::layer().overlay(true).build(ui, |ui| {
                        nuon::translate()
                            .x((body_w - 70.0) * ((i + 1) as f32 / values.len() as f32) - w / 2.0)
                            .y(-28.0)
                            .build(ui, |ui| {
                                nuon::quad()
                                    .size(w, 24.0)
                                    .color(theme::surface())
                                    .border_color(theme::border_strong())
                                    .border_width(1.0)
                                    .border_radius([theme::RADIUS_S; 4])
                                    .build(ui);
                                nuon::label()
                                    .size(w, 24.0)
                                    .font_size(theme::FONT_S - 1.0)
                                    .color(theme::text())
                                    .text(text)
                                    .build(ui);
                            });
                    });
                }
            } else {
                widgets::hint(ui, body_w, 40.0, &locals::song::empty_takes());
            }
        });

        // ── Gravações ────────────────────────────────────────────────
        // +26: espaço para as datas da evolução por baixo do gráfico
        let list_y = evo_y + 34.0 + 42.0 + 140.0 + 26.0 + theme::SPACE_XL;
        let list_h = (win_h - list_y - theme::SPACE_L - theme::BUTTON_H).max(120.0);
        nuon::translate().x(x0).y(list_y).build(ui, |ui| {
            widgets::section_title(ui, body_w, &locals::song::recordings(), Some(takes.len()));
            nuon::translate().y(34.0).add_to_current(ui);

            if takes.is_empty() {
                widgets::hint(ui, body_w, 60.0, &locals::song::empty_takes());
            } else {
                nuon::scroll("song_takes")
                    .scissor_size(body_w, list_h)
                    .build(ui, |ui| {
                        let row_w = body_w - 14.0;
                        let trailing = 36.0 * 2.0 + theme::SPACE_S;
                        for (i, t) in takes.iter().enumerate() {
                            let confirming = {
                                let ps = page_state(ui);
                                ps.pending_delete == Some(i)
                                    && ps
                                        .pending_delete_at
                                        .is_some_and(|t0| t0.elapsed().as_secs_f32() < 3.0)
                            };

                            nuon::translate().build(ui, |ui| {
                                nuon::quad()
                                    .size(row_w, 68.0)
                                    .color(theme::surface_sunken())
                                    .border_color(theme::border())
                                    .border_width(1.0)
                                    .border_radius([theme::RADIUS_M; 4])
                                    .build(ui);

                                // Linha 1: precisão + data + "completa"
                                nuon::label()
                                    .pos(theme::SPACE_L, 8.0)
                                    .size(row_w - trailing - theme::SPACE_L * 2.0, 20.0)
                                    .font_size(theme::FONT_S)
                                    .bold(true)
                                    .color(theme::text())
                                    .text_justify(TextAlign::Start)
                                    .text(format!(
                                        "{:.0} % · {}{}",
                                        t.accuracy * 100.0,
                                        t.date_label(),
                                        if is_complete(t, dur) {
                                            format!(" · {}", locals::song::complete())
                                        } else {
                                            String::new()
                                        }
                                    ))
                                    .build(ui);

                                // Barra do trecho tocado (sobre a duração)
                                let bar_w = row_w - trailing - theme::SPACE_L * 2.0;
                                nuon::quad()
                                    .pos(theme::SPACE_L, 32.0)
                                    .size(bar_w, 6.0)
                                    .color(theme::surface_active())
                                    .border_radius([3.0; 4])
                                    .build(ui);
                                if let Some((a, b)) = t.info.covered
                                    && dur > 0.0
                                {
                                    let x = (a.as_secs_f32() / dur).clamp(0.0, 1.0);
                                    let w = ((b.as_secs_f32() - a.as_secs_f32()) / dur)
                                        .clamp(0.0, 1.0 - x);
                                    nuon::quad()
                                        .pos(theme::SPACE_L + bar_w * x, 32.0)
                                        .size((bar_w * w).max(2.0), 6.0)
                                        .color(theme::accent())
                                        .border_radius([3.0; 4])
                                        .build(ui);
                                }

                                // Linha 2: certas · andamento · hesitações
                                nuon::label()
                                    .pos(theme::SPACE_L, 44.0)
                                    .size(bar_w, 20.0)
                                    .font_size(theme::FONT_S - 1.0)
                                    .color(theme::text_muted())
                                    .text_justify(TextAlign::Start)
                                    .text(format!(
                                        "{} {} · {} {} · {}",
                                        t.hits,
                                        locals::song::stat_hits(),
                                        t.info
                                            .tempo_ratio
                                            .map(|r| format!("{:.0} %", r * 100.0))
                                            .unwrap_or_else(|| "—".into()),
                                        locals::song::stat_tempo(),
                                        locals::song::stat_hesitations(t.info.hesitations)
                                    ))
                                    .build(ui);

                                // Trailing: ▶ ver + × apagar
                                nuon::translate().x(row_w - trailing).build(ui, |ui| {
                                    if widgets::button(("take_view", i), "")
                                        .icon(icons::play_icon())
                                        .ghost()
                                        .size(36.0, 36.0)
                                        .tooltip(locals::song::view())
                                        .build(ui)
                                        .clicked
                                    {
                                        act = Some(Act::OpenTake(i));
                                    }
                                    nuon::translate()
                                        .x(36.0 + theme::SPACE_S)
                                        .add_to_current(ui);

                                    let mut b = if confirming {
                                        widgets::button(
                                            ("take_del", i),
                                            locals::song::confirm_delete(),
                                        )
                                        .danger()
                                        .size(110.0, 36.0)
                                        .font_size(theme::FONT_S - 1.0)
                                    } else {
                                        widgets::button(("take_del", i), "")
                                            .icon(icons::trash_icon())
                                            .size(36.0, 36.0)
                                            .tooltip(locals::song::delete())
                                    };
                                    b = b.danger();
                                    if b.build(ui).clicked {
                                        if confirming {
                                            act = Some(Act::DeleteTake(i));
                                            let ps = page_state(ui);
                                            ps.pending_delete = None;
                                        } else {
                                            let ps = page_state(ui);
                                            ps.pending_delete = Some(i);
                                            ps.pending_delete_at = Some(std::time::Instant::now());
                                        }
                                    }
                                });
                            });

                            nuon::translate()
                                .y(68.0 + theme::SPACE_S)
                                .add_to_current(ui);
                        }
                    });
            }
        });

        self.apply_nav_actions(ctx, nav_act);

        // ── Aplicar ação ─────────────────────────────────────────────
        let Some(act) = act else { return };
        match act {
            Act::Play => {
                play_path(&self.state, ctx, song, None);
            }
            Act::OpenTake(i) => {
                if let Some(t) = self.state.recordings.get(i)
                    && let Ok(midi) = MidiFile::new(&t.midi_path)
                {
                    super::state::connect_io(&self.state, ctx);
                    ctx.proxy
                        .send_event(crate::NeothesiaEvent::Play(crate::song::Song::new(midi)))
                        .ok();
                }
            }
            Act::DeleteTake(i) => {
                if let Some(t) = self.state.recordings.get(i).cloned() {
                    if let Err(e) = ctx.takes.delete(&t) {
                        log::error!("Falha ao apagar a gravação: {e}");
                    }
                    self.state.reload_recordings(ctx);
                }
            }
        }
    }
}
