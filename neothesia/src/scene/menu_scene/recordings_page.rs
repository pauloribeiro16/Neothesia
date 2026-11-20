//! Página "Gravações" de uma música: lista, evolução, ver e apagar.
//! Versão inicial do A; o redesenho final é a tarefa B8.

use midi_file::MidiFile;

use crate::{
    NeothesiaEvent, context::Context, icons, library::Library, song::Song, theme, widgets,
};

use super::song_page::is_complete;

enum Act {
    Back,
    View(usize),
    Delete(usize),
}

impl super::MenuScene {
    pub fn recordings_page_window_event(
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

    pub fn recordings_page_ui(&mut self, ctx: &mut Context, ui: &mut nuon::Ui) {
        let win_w = ctx.window_state.logical_size.width;
        let win_h = ctx.window_state.logical_size.height;
        let margin = theme::SPACE_XL;
        let body_w = (win_w - margin * 2.0).min(860.0);
        let x0 = (win_w - body_w) / 2.0;
        let mut act: Option<Act> = None;

        let (title, composer) = self
            .state
            .recordings_song
            .as_deref()
            .map(Library::pretty_title)
            .unwrap_or_default();
        let takes = &self.state.recordings;

        nuon::translate().x(x0).y(margin).build(ui, |ui| {
            nuon::label()
                .size(body_w, 36.0)
                .font_size(theme::FONT_XL)
                .bold(true)
                .color(theme::text())
                .text_justify(nuon::TextAlign::Start)
                .text(theme::fit_text(&format!("Gravações · {title}"), body_w, theme::FONT_XL))
                .build(ui);
            if let Some(c) = &composer {
                nuon::label()
                    .pos(0.0, 36.0)
                    .size(body_w, 20.0)
                    .font_size(theme::FONT_S)
                    .color(theme::text_muted())
                    .text_justify(nuon::TextAlign::Start)
                    .text(c)
                    .build(ui);
            }
            nuon::translate().y(64.0).add_to_current(ui);

            if takes.is_empty() {
                widgets::hint(
                    ui,
                    body_w,
                    120.0,
                    "Ainda não há gravações. Toca esta música e a gravação aparece aqui.",
                );
                return;
            }

            // Evolução (mais antiga → mais recente)
            widgets::section_title(ui, body_w, "Evolução", None);
            nuon::translate().y(34.0).add_to_current(ui);
            let acc: Vec<f32> = takes.iter().rev().map(|t| t.accuracy).collect();
            let chart_h = 150.0;
            if let Some(i) = widgets::line_chart(ui, "rec_line", body_w, chart_h, &acc) {
                let t = &takes[takes.len() - 1 - i];
                nuon::label()
                    .pos(0.0, chart_h + 2.0)
                    .size(body_w, 20.0)
                    .font_size(theme::FONT_S)
                    .color(theme::text_muted())
                    .text(format!("{} · {:.0}%", t.date_label(), t.accuracy * 100.0))
                    .build(ui);
            }
            nuon::translate().y(chart_h + theme::SPACE_XL).add_to_current(ui);

            widgets::section_title(ui, body_w, "Todas", Some(takes.len()));
            nuon::translate().y(34.0).add_to_current(ui);

            let list_h = (win_h - margin * 3.0 - 52.0 - 34.0 - chart_h - theme::SPACE_XL - 34.0 - 60.0)
                .max(theme::ROW_H);
            nuon::scroll("rec_scroll")
                .scissor_size(body_w, list_h)
                .build(ui, |ui| {
                    let row_w = body_w - 14.0;
                    let trailing = 36.0 * 2.0 + theme::SPACE_S * 3.0;
                    let best = takes
                        .iter()
                        .filter(|t| is_complete(t, takes))
                        .map(|t| t.accuracy)
                        .fold(0.0, f32::max);
                    for (i, t) in takes.iter().enumerate() {
                        let mut subtitle = format!(
                            "{} certas · {} falhadas · {} a mais",
                            t.hits, t.missed, t.extra
                        );
                        if let Some((a, b)) = t.info.covered {
                            subtitle.push_str(&format!(
                                " · trecho {}:{:02}–{}:{:02}",
                                a.as_secs() / 60,
                                a.as_secs() % 60,
                                b.as_secs() / 60,
                                b.as_secs() % 60
                            ));
                        }
                        if let Some(r) = t.info.tempo_ratio {
                            subtitle.push_str(&format!(" · andamento {:.0}%", r * 100.0));
                        }
                        subtitle.push_str(&format!(
                            " · {} hesitações · {}:{:02}",
                            t.info.hesitations,
                            t.duration.as_secs() / 60,
                            t.duration.as_secs() % 60
                        ));
                        let title = format!(
                            "{:.0}%   {}{}",
                            t.accuracy * 100.0,
                            t.date_label(),
                            if t.accuracy == best && is_complete(t, takes) { "   ★ melhor" } else { "" }
                        );
                        nuon::translate().build(ui, |ui| {
                            if widgets::list_row(("rec_row", i), row_w, &title)
                                .subtitle(&subtitle)
                                .active(i == 0)
                                .trailing(trailing)
                                .build(ui)
                                .clicked
                            {
                                act = Some(Act::View(i));
                            }
                            nuon::translate()
                                .x(row_w - trailing + theme::SPACE_S)
                                .y((theme::ROW_H - 36.0) / 2.0)
                                .build(ui, |ui| {
                                    if widgets::button(("rec_view", i), "")
                                        .icon(icons::play_icon())
                                        .ghost()
                                        .size(36.0, 36.0)
                                        .tooltip("Ver gravação")
                                        .build(ui)
                                        .clicked
                                    {
                                        act = Some(Act::View(i));
                                    }
                                    nuon::translate().x(36.0 + theme::SPACE_S).add_to_current(ui);
                                    if widgets::button(("rec_del", i), "")
                                        .icon(icons::trash_icon())
                                        .danger()
                                        .size(36.0, 36.0)
                                        .tooltip("Apagar")
                                        .build(ui)
                                        .clicked
                                    {
                                        act = Some(Act::Delete(i));
                                    }
                                });
                        });
                        nuon::translate().y(theme::ROW_H + theme::SPACE_S).add_to_current(ui);
                    }
                });
        });

        let bar_h = theme::BUTTON_H + 8.0;
        nuon::translate().x(margin).y(win_h - margin - bar_h).build(ui, |ui| {
            if widgets::icon_button(ui, "rec_back", icons::left_arrow_icon(), bar_h).clicked {
                act = Some(Act::Back);
            }
        });

        match act {
            Some(Act::Back) => self.state.go_back(),
            Some(Act::View(i)) => {
                if let Some(take) = self.state.recordings.get(i)
                    && let Ok(midi) = MidiFile::new(&take.midi_path)
                {
                    super::state::connect_io(&self.state, ctx);
                    ctx.proxy.send_event(NeothesiaEvent::Play(Song::new(midi))).ok();
                }
            }
            Some(Act::Delete(i)) => {
                if let Some(take) = self.state.recordings.get(i).cloned() {
                    if let Err(e) = ctx.takes.delete(&take) {
                        log::error!("Falha ao apagar a gravação: {e}");
                    }
                    self.state.reload_recordings(ctx);
                }
            }
            None => {}
        }
    }
}
