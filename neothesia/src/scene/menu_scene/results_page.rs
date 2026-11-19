//! Página "Resultado": mostrada ao voltar do ecrã de tocar quando houve gravação.
//! Versão inicial do A; o redesenho final é a tarefa B7.

use crate::{
    context::Context, icons, scene::menu_scene::library_page::play_path, theme, widgets,
};

enum Act {
    Back,
    Recordings,
    Replay,
    Next,
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
        let win_w = ctx.window_state.logical_size.width;
        let win_h = ctx.window_state.logical_size.height;

        let Some(result) = self.state.result.clone() else {
            self.state.go_back();
            return;
        };
        let (take, report) = (&result.take, &result.report);
        let mut act: Option<Act> = None;

        let margin = theme::SPACE_XL;
        let body_w = (win_w - margin * 2.0).min(860.0);
        let x0 = (win_w - body_w) / 2.0;
        let title = crate::library::Library::display_name(&take.song);

        nuon::translate().x(x0).y(margin).build(ui, |ui| {
            // Cabeçalho
            nuon::label()
                .size(body_w, 36.0)
                .font_size(theme::FONT_XL)
                .bold(true)
                .color(theme::text())
                .text_justify(nuon::TextAlign::Start)
                .text(theme::fit_text(&title, body_w * 0.7, theme::FONT_XL))
                .build(ui);
            nuon::label()
                .size(body_w, 36.0)
                .font_size(theme::FONT_S)
                .color(theme::text_muted())
                .text_justify(nuon::TextAlign::End)
                .text(format!(
                    "{} · {}:{:02}",
                    take.date_label(),
                    take.duration.as_secs() / 60,
                    take.duration.as_secs() % 60
                ))
                .build(ui);

            nuon::translate().y(56.0).add_to_current(ui);

            // Precisão grande, a contar até ao valor
            let t = theme::ease_out(theme::anim(ui, "result_count", 1.0, 0.05));
            let pct = (report.accuracy * 100.0 * t).round();
            let card_h = 150.0;
            theme::panel(ui, body_w, card_h);
            nuon::label()
                .pos(0.0, 18.0)
                .size(body_w * 0.4, 80.0)
                .font_size(64.0)
                .bold(true)
                .color(theme::accent().mix(theme::accent_2(), report.accuracy))
                .text(format!("{pct:.0}%"))
                .build(ui);
            nuon::label()
                .pos(0.0, 98.0)
                .size(body_w * 0.4, 24.0)
                .font_size(theme::FONT_S)
                .color(theme::text_muted())
                .text("notas certas")
                .build(ui);

            let stat_w = body_w * 0.6 / 3.0;
            for (i, (n, label, color)) in [
                (report.hits, "certas", theme::text()),
                (report.missed, "falhadas", theme::danger()),
                (report.extra, "a mais", theme::text_muted()),
            ]
            .into_iter()
            .enumerate()
            {
                let x = body_w * 0.4 + stat_w * i as f32;
                nuon::label()
                    .pos(x, 34.0)
                    .size(stat_w, 44.0)
                    .font_size(theme::FONT_XL)
                    .bold(true)
                    .color(color)
                    .text(((n as f32 * t).round() as usize).to_string())
                    .build(ui);
                nuon::label()
                    .pos(x, 80.0)
                    .size(stat_w, 22.0)
                    .font_size(theme::FONT_S)
                    .color(theme::text_muted())
                    .text(label)
                    .build(ui);
            }
            nuon::label()
                .pos(body_w * 0.4, 110.0)
                .size(body_w * 0.6, 22.0)
                .font_size(theme::FONT_S)
                .color(theme::text_muted())
                .text(format!(
                    "Ritmo: {:+.0} ms ({}) · regularidade {:.0} ms",
                    report.mean_offset_ms,
                    report.timing_label(),
                    report.offset_spread_ms
                ))
                .build(ui);

            nuon::translate().y(card_h + theme::SPACE_XL).add_to_current(ui);

            // Erros por compasso
            widgets::section_title(ui, body_w, "Erros por compasso", None);
            nuon::translate().y(34.0).add_to_current(ui);
            let errors: Vec<f32> = report.sections.iter().map(|s| s.errors() as f32).collect();
            let chart_h = (win_h - 56.0 - card_h - 34.0 - margin * 4.0 - 70.0).clamp(80.0, 220.0);
            let hovered = widgets::bar_chart(ui, "result_bars", body_w, chart_h, &errors, theme::danger());
            if let Some(i) = hovered {
                let s = &report.sections[i];
                widgets::hint(
                    ui,
                    body_w,
                    -26.0,
                    &format!("Compasso {}: {} falhadas, {} a mais", i + 1, s.missed, s.extra),
                );
            }
        });

        // Barra inferior
        let bar_h = theme::BUTTON_H + 8.0;
        nuon::translate().x(margin).y(win_h - margin - bar_h).build(ui, |ui| {
            if widgets::icon_button(ui, "res_back", icons::left_arrow_icon(), bar_h).clicked {
                act = Some(Act::Back);
            }
            let right = win_w - margin * 2.0;
            let has_next = ctx.library.has_next();
            let mut x = right;
            if has_next {
                x -= 170.0;
                nuon::translate().x(x).build(ui, |ui| {
                    if widgets::button("res_next", "Seguinte")
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
                let b = widgets::button("res_replay", "Repetir").size(150.0, bar_h);
                let b = if has_next { b } else { b.primary() };
                if b.build(ui).clicked {
                    act = Some(Act::Replay);
                }
            });
            x -= theme::SPACE_M + 160.0;
            nuon::translate().x(x).build(ui, |ui| {
                if widgets::button("res_recs", "Gravações")
                    .size(160.0, bar_h)
                    .build(ui)
                    .clicked
                {
                    act = Some(Act::Recordings);
                }
            });
        });

        match act {
            Some(Act::Back) => self.state.go_back(),
            Some(Act::Recordings) => self.state.open_recordings(ctx, take.song.clone()),
            Some(Act::Replay) => super::state::play(&self.state, ctx),
            Some(Act::Next) => {
                if let Some(path) = ctx.library.advance() {
                    let pos = ctx.library.queue_pos;
                    play_path(&self.state, ctx, path, pos);
                }
            }
            None => {}
        }
    }
}
