//! Página principal do menu.
//!
//! Layout (ver `.agents/TAREFAS.md` › B5): logótipo com a animação de entrada,
//! subtítulo, cartão "Continuar" com a última música, quatro cartões de ação
//! com atalhos (Biblioteca [L], Abrir ficheiro [O], Tocar livre [F], Definições
//! [S], em grelha 2×2 se a largura < 900) e a lista "Recentes". Sair no canto.

use std::path::PathBuf;

use super::{open_midi_file_picker, state, MsgFn, Page, UiState, ease_out};
use crate::{context::Context, icons, library::Library, locals, theme, utils::BoxFuture, widgets};

/// Glifo bootstrap "folder" (a mover para `icons.rs`)
const FOLDER_ICON: &str = "\u{F3D7}";

const CARD_W: f32 = 200.0;
const CARD_H: f32 = 120.0;
const WIDE_W: f32 = 520.0;

enum Act {
    PlayLast,
    Play(PathBuf),
    OpenFile,
    Library,
    Settings,
    Freeplay,
    Tracks,
    Exit,
}

impl super::MenuScene {
    pub fn main_page_ui(&mut self, ctx: &mut Context, ui: &mut nuon::Ui) {
        let win_w = ctx.window_state.logical_size.width;
        let win_h = ctx.window_state.logical_size.height;

        let mut act: Option<Act> = None;

        nuon::translate()
            .x(win_w / 2.0)
            .y(win_h / 8.0)
            .build(ui, |ui| {
                // ── Logótipo: cai suavemente e depois "flutua" ──────────
                let logo_w = 420.0;
                let logo_h = logo_w * (118.0 / 650.0);
                let logo_p = ease_out(self.age / 0.6);
                let float = (self.age * 1.4).sin() * 3.0 * logo_p;
                nuon::image(self.logo)
                    .x(-logo_w / 2.0)
                    .y(-(1.0 - logo_p) * 70.0 + float)
                    .size(logo_w, logo_h)
                    .build(ui);

                // Subtítulo
                nuon::translate().y(logo_h).add_to_current(ui);
                nuon::label()
                    .x(-220.0)
                    .size(440.0, 28.0)
                    .font_size(theme::FONT_M)
                    .color(theme::text_muted())
                    .text(locals::menu::subtitle())
                    .build(ui);
                nuon::translate().y(28.0 + theme::SPACE_L).add_to_current(ui);

                // ── Cartão "Continuar" ──────────────────────────────────
                if self.state.song().is_some() {
                    let drop = (1.0 - ease_out((self.age - 0.15) / 0.5)) * 24.0;
                    nuon::translate().y(drop).build(ui, |ui| {
                        nuon::translate().x(-WIDE_W / 2.0).build(ui, |ui| {
                            nuon::label()
                                .size(WIDE_W, 20.0)
                                .font_size(theme::FONT_S)
                                .color(theme::text_muted())
                                .text_justify(nuon::TextAlign::Start)
                                .text(locals::menu::continue_card())
                                .build(ui);
                            nuon::translate().y(20.0 + theme::SPACE_XS).add_to_current(ui);

                            let song_name = self
                                .state
                                .song()
                                .map(|s| s.file.name.clone())
                                .unwrap_or_default();

                            let r = widgets::card(
                                ui,
                                ("main_continue", 0),
                                WIDE_W,
                                64.0,
                                |ui, t| {
                                    nuon::label()
                                        .x(theme::SPACE_L)
                                        .size(40.0, 64.0)
                                        .font_size(22.0)
                                        .color(theme::accent().mix(theme::text(), t * 0.4))
                                        .icon(icons::play_icon())
                                        .build(ui);
                                    nuon::label()
                                        .x(56.0)
                                        .size(WIDE_W - 56.0 - 96.0 - theme::SPACE_L * 2.0, 64.0)
                                        .font_size(theme::FONT_M)
                                        .color(theme::text())
                                        .text_justify(nuon::TextAlign::Start)
                                        .text(theme::fit_text(
                                            &song_name,
                                            WIDE_W - 56.0 - 96.0 - theme::SPACE_L * 2.0,
                                            theme::FONT_M,
                                        ))
                                        .build(ui);

                                    nuon::translate()
                                        .x(WIDE_W - 96.0 - theme::SPACE_L)
                                        .y((64.0 - 36.0) / 2.0)
                                        .build(ui, |ui| {
                                            if widgets::button(
                                                ("main_continue_play", 0),
                                                locals::menu::play(),
                                            )
                                            .primary()
                                            .size(96.0, 36.0)
                                            .font_size(theme::FONT_S)
                                            .build(ui)
                                            .clicked
                                            {
                                                act = Some(Act::PlayLast);
                                            }
                                        });
                                },
                            );
                            if r.clicked {
                                act = Some(Act::PlayLast);
                            }
                        });
                    });
                    // Avançar fora do bloco com âmbito: dentro dele o avanço perdia-se
                    // e os cartões de ação ficavam por cima do "Continuar".
                    nuon::translate().y(20.0 + theme::SPACE_XS + 64.0 + theme::SPACE_L).add_to_current(ui);
                }

                // ── Cartões de ação (4 em linha; 2×2 se largura < 900) ──
                let cards = [
                    (locals::library::title(), icons::note_list_icon(), "L"),
                    (locals::menu::open_file(), FOLDER_ICON, "O"),
                    (locals::menu::freeplay(), icons::balloon_icon(), "F"),
                    (locals::menu::settings(), icons::gear_icon(), "S"),
                ];
                let cols = if win_w < 900.0 { 2 } else { 4 };
                let gap = theme::SPACE_M;
                let row_w = cols as f32 * CARD_W + (cols as f32 - 1.0) * gap;

                // Entrada em cascata, alternando esquerda/direita (i = cartão)
                let age = self.age;
                let slide = move |i: usize| {
                    let p = ease_out((age - 0.25 - i as f32 * 0.09) / 0.5);
                    let dir = if i % 2 == 0 { -1.0 } else { 1.0 };
                    dir * (1.0 - p) * 260.0
                };

                nuon::translate().x(-row_w / 2.0).build(ui, |ui| {
                    for (i, (label, icon, key)) in cards.iter().enumerate() {
                        let col = i % cols;
                        if i > 0 && col == 0 {
                            nuon::translate()
                                .x(-(cols as f32) * (CARD_W + gap))
                                .add_to_current(ui);
                            nuon::translate().y(CARD_H + gap).add_to_current(ui);
                        }

                        nuon::translate().x(slide(i)).build(ui, |ui| {
                            let r = widgets::card(ui, ("main_card", i), CARD_W, CARD_H, |ui, t| {
                                nuon::label()
                                    .x(theme::SPACE_L)
                                    .y(theme::SPACE_L)
                                    .size(40.0, 40.0)
                                    .font_size(28.0)
                                    .color(theme::accent().mix(theme::accent_2(), t))
                                    .icon(*icon)
                                    .build(ui);
                                nuon::label()
                                    .x(theme::SPACE_L)
                                    .y(CARD_H - 22.0 - theme::SPACE_L - 24.0)
                                    .size(CARD_W - theme::SPACE_L * 2.0, 24.0)
                                    .font_size(theme::FONT_M)
                                    .color(theme::text())
                                    .text_justify(nuon::TextAlign::Start)
                                    .text(theme::fit_text(
                                        label,
                                        CARD_W - theme::SPACE_L * 2.0,
                                        theme::FONT_M,
                                    ))
                                    .build(ui);

                                nuon::translate()
                                    .x(CARD_W - 33.0 - theme::SPACE_M)
                                    .y(CARD_H - 22.0 - theme::SPACE_M)
                                    .build(ui, |ui| {
                                        widgets::kbd(ui, key);
                                    });
                            });
                            if r.clicked {
                                act = match i {
                                    0 => Some(Act::Library),
                                    1 => Some(Act::OpenFile),
                                    2 => Some(Act::Freeplay),
                                    _ => Some(Act::Settings),
                                };
                            }
                        });

                        nuon::translate().x(CARD_W + gap).add_to_current(ui);
                    }
                });

                nuon::translate().y(CARD_H + theme::SPACE_XL).add_to_current(ui);

                // ── Recentes (até 5, a 1 clique) ────────────────────────
                let recent: Vec<_> = ctx.library.recent.iter().take(5).collect();
                if !recent.is_empty() {
                    nuon::translate().x(-WIDE_W / 2.0).build(ui, |ui| {
                        widgets::section_title(ui, WIDE_W, &locals::library::recent(), None);
                        nuon::translate().y(28.0 + theme::SPACE_S).add_to_current(ui);

                        for (i, path) in recent.iter().enumerate() {
                            let title = Library::display_name(path);
                            let r = widgets::list_row(("main_recent", i), WIDE_W, &title)
                                .height(44.0)
                                .build(ui);
                            if r.clicked {
                                act = Some(Act::Play((*path).clone()));
                            }
                            nuon::translate().y(44.0 + theme::SPACE_XS).add_to_current(ui);
                        }
                    });
                }
            });

        // ── Canto inferior direito: Sair (e Faixas, se houver música) ──
        nuon::translate().x(win_w).y(win_h).build(ui, |ui| {
            nuon::translate()
                .x(-theme::SPACE_L - 110.0)
                .y(-theme::SPACE_L - 44.0)
                .build(ui, |ui| {
                    if widgets::button(("main_exit", 0), locals::menu::exit())
                        .ghost()
                        .size(110.0, 44.0)
                        .build(ui)
                        .clicked
                    {
                        act = Some(Act::Exit);
                    }

                    if self.state.song().is_some() {
                        nuon::translate().x(-44.0 - theme::SPACE_S).add_to_current(ui);
                        if widgets::button(("main_tracks", 0), "")
                            .icon(icons::note_list_icon())
                            .size(44.0, 44.0)
                            .ghost()
                            .tooltip(locals::menu::tracks())
                            .build(ui)
                            .clicked
                        {
                            act = Some(Act::Tracks);
                        }
                    }
                });
        });

        // ── Aplicar ação ────────────────────────────────────────────
        let Some(act) = act else { return };
        match act {
            Act::PlayLast => state::play(&self.state, ctx),
            Act::Play(path) => {
                super::library_page::play_path(&self.state, ctx, path, None);
            }
            Act::OpenFile => self.futures.push(open_midi_file_picker(&mut self.state, ctx)),
            Act::Library => self.state.go_to(Page::Library),
            Act::Settings => self.state.go_to(Page::Settings),
            Act::Freeplay => state::freeplay(&self.state, ctx),
            Act::Tracks => self.state.go_to(Page::TrackSelection),
            Act::Exit => self.state.go_back(),
        }
    }
}
