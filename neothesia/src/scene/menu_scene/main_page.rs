//! Página principal do menu.

use super::{Page, ease_out, neo_btn, open_midi_file_picker, state};
use crate::{context::Context, icons, locals};

impl super::MenuScene {
    pub fn main_page_ui(&mut self, ctx: &mut Context, ui: &mut nuon::Ui) {
        let win_w = ctx.window_state.logical_size.width;
        let win_h = ctx.window_state.logical_size.height;

        let w = 450.0;
        let h = 80.0;
        let gap = 10.0;

        let logo_w = 650.0;
        let logo_h = 118.0;
        let post_logo_gap = 40.0;

        nuon::translate()
            .x(win_w / 2.0)
            .y(win_h / 5.0)
            .build(ui, |ui| {
                // Logo: cai suavemente e depois "flutua"
                let logo_p = ease_out(self.age / 0.6);
                let float = (self.age * 1.4).sin() * 3.0 * logo_p;
                nuon::image(self.logo)
                    .x(-logo_w / 2.0)
                    .y(-(1.0 - logo_p) * 70.0 + float)
                    .size(logo_w, logo_h)
                    .build(ui);

                let age = self.age;
                // Botões entram em cascata, alternando esquerda/direita
                let slide = move |i: usize| {
                    let p = ease_out((age - 0.25 - i as f32 * 0.09) / 0.5);
                    let dir = if i % 2 == 0 { -1.0 } else { 1.0 };
                    dir * (1.0 - p) * 260.0
                };

                nuon::translate()
                    .x(-w / 2.0)
                    .y(logo_h + post_logo_gap)
                    .build(ui, |ui| {
                        nuon::translate().x(slide(0)).build(ui, |ui| {
                            if neo_btn()
                                .size(w, h)
                                .label(locals::menu::select_file())
                                .build(ui)
                            {
                                self.futures
                                    .push(open_midi_file_picker(&mut self.state, ctx));
                            }
                        });

                        nuon::translate().y(h + gap).add_to_current(ui);

                        nuon::translate().x(slide(1)).build(ui, |ui| {
                            if neo_btn().size(w, h).label("Biblioteca").build(ui) {
                                self.state.go_to(Page::Library);
                            }
                        });

                        nuon::translate().y(h + gap).add_to_current(ui);

                        nuon::translate().x(slide(2)).build(ui, |ui| {
                            if neo_btn()
                                .size(w, h)
                                .label(locals::menu::settings())
                                .build(ui)
                            {
                                self.state.go_to(Page::Settings);
                            }
                        });

                        nuon::translate().y(h + gap).add_to_current(ui);

                        nuon::translate().x(slide(3)).build(ui, |ui| {
                            if neo_btn().size(w, h).label(locals::menu::exit()).build(ui) {
                                self.state.go_back();
                            }
                        });
                    });
            });

        nuon::translate().x(0.0).y(win_h).build(ui, |ui| {
            let gap = 10.0;
            let btn_w = 80.0;
            let btn_h = 60.0;

            nuon::translate().y(-gap).add_to_current(ui);
            nuon::translate().y(-btn_h).add_to_current(ui);

            if let Some(song) = self.state.song() {
                nuon::label()
                    .text(&song.file.name)
                    .size(win_w, 60.0)
                    .font_size(16.0)
                    .build(ui);
            }

            nuon::translate().build(ui, |ui| {
                nuon::translate().x(gap).add_to_current(ui);

                if neo_btn()
                    .size(btn_w, btn_h)
                    .icon(icons::balloon_icon())
                    .color([100; 3])
                    .tooltip(locals::menu::freeplay())
                    .build(ui)
                {
                    state::freeplay(&self.state, ctx);
                }
            });

            if self.state.song().is_none() {
                return;
            }

            nuon::translate().x(win_w).build(ui, |ui| {
                nuon::translate().x(-btn_w - gap).add_to_current(ui);

                if neo_btn()
                    .size(btn_w, btn_h)
                    .icon(icons::play_icon())
                    .tooltip(locals::menu::play())
                    .build(ui)
                {
                    state::play(&self.state, ctx);
                }

                nuon::translate().x(-btn_w - gap).add_to_current(ui);

                if neo_btn()
                    .size(btn_w, btn_h)
                    .icon(icons::note_list_icon())
                    .tooltip(locals::menu::tracks())
                    .build(ui)
                {
                    self.state.go_to(Page::TrackSelection);
                }
            });
        });
    }
}
