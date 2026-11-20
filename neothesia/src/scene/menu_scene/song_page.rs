//! Página de uma música (maqueta "Página da música"): título e autor, os números
//! da melhor gravação, evolução e lista de gravações.
//!
//! Versão MÍNIMA do A só com o encaminhamento. O desenho final é a tarefa R3 do B
//! (ver `.agents/TAREFAS.md`). Dados: `self.state.song_page`, `self.state.recordings`.

use crate::{context::Context, icons, library::Library, theme, widgets};

impl super::MenuScene {
    pub fn song_page_window_event(&mut self, _ctx: &mut Context, event: &winit::event::WindowEvent) {
        use crate::utils::window::WinitEvent;
        use winit::keyboard::{Key, NamedKey};
        if event.key_pressed(Key::Named(NamedKey::Escape)) {
            self.state.go_back();
        }
    }

    pub fn song_page_ui(&mut self, ctx: &mut Context, ui: &mut nuon::Ui) {
        let win_w = ctx.window_state.logical_size.width;
        let win_h = ctx.window_state.logical_size.height;
        let Some(song) = self.state.song_page.clone() else {
            self.state.go_back();
            return;
        };
        let (title, composer) = Library::pretty_title(&song);
        let m = theme::SPACE_XL;
        nuon::translate().x(m).y(m).build(ui, |ui| {
            widgets::page_title(ui, win_w - m * 2.0, &title);
            nuon::translate().y(56.0).add_to_current(ui);
            widgets::hint(ui, win_w - m * 2.0, 24.0, composer.as_deref().unwrap_or(""));
            nuon::translate().y(40.0).add_to_current(ui);
            widgets::hint(
                ui,
                win_w - m * 2.0,
                24.0,
                &format!("{} gravações · página em construção (tarefa R3)", self.state.recordings.len()),
            );
        });
        let bar_h = theme::BUTTON_H + 8.0;
        let mut back = false;
        nuon::translate().x(m).y(win_h - m - bar_h).build(ui, |ui| {
            back = widgets::icon_button(ui, "song_back", icons::left_arrow_icon(), bar_h).clicked;
        });
        if back {
            self.state.go_back();
        }
    }
}
