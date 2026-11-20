//! Página "Início" (R4): navegação lateral, "Neothasia" em serifa, cartão
//! "Continuar" com a última música (título limpo), ações Abrir ficheiro /
//! Tocar livre e a lista "Recentes" com autor. Sair no canto do conteúdo.

use std::path::PathBuf;

use super::{open_midi_file_picker, state, MsgFn, UiState, ease_out};
use crate::{context::Context, icons, library::Library, locals, theme, utils::BoxFuture, widgets};

/// Glifo bootstrap "folder" (a mover para `icons.rs`)
const FOLDER_ICON: &str = "\u{F3D7}";

const WIDE_W: f32 = 520.0;

enum Act {
    PlayLast,
    OpenSong(PathBuf),
    OpenFile,
    Freeplay,
    Exit,
}

impl super::MenuScene {
    pub fn main_page_ui(&mut self, ctx: &mut Context, ui: &mut nuon::Ui) {
        let win_h = ctx.window_state.logical_size.height;

        let (nav, nav_act) = self.side_nav(ctx, ui);
        let mut act: Option<Act> = None;

        let content_w = nav.content_w;
        let cx = nav.content_x + content_w / 2.0;
        let x0 = nav.content_x + theme::SPACE_XL;

        // ── "Neothasia" em serifa, com entrada suave e flutuação ────────
        let y_title = win_h * 0.12;
        nuon::translate().x(cx).y(y_title).build(ui, |ui| {
            let logo_p = ease_out(self.age / 0.6);
            let drop = (1.0 - logo_p) * 40.0;
            let float = (self.age * 1.4).sin() * 3.0 * logo_p;
            nuon::label()
                .x(-220.0)
                .y(-drop + float)
                .size(440.0, 64.0)
                .font_size(56.0)
                .font_family(theme::FONT_DISPLAY)
                .bold(true)
                .color(theme::text())
                .text("Neothasia")
                .build(ui);
            nuon::label()
                .x(-220.0)
                .y(64.0 + drop)
                .size(440.0, 26.0)
                .font_size(theme::FONT_M)
                .color(theme::text_muted())
                .text(locals::menu::subtitle())
                .build(ui);
        });

        // ── Cartão "Continuar" ─────────────────────────────────────────
        let mut y = y_title + 64.0 + 26.0 + theme::SPACE_XL;
        if self.state.song().is_some() {
            let drop = (1.0 - ease_out((self.age - 0.15) / 0.5)) * 24.0;
            nuon::translate().x(cx - WIDE_W / 2.0).y(y + drop).build(ui, |ui| {
                nuon::label()
                    .size(WIDE_W, 20.0)
                    .font_size(theme::FONT_S)
                    .color(theme::text_muted())
                    .text_justify(nuon::TextAlign::Start)
                    .text(locals::menu::continue_card())
                    .build(ui);

                let song_name = self
                    .state
                    .song()
                    .map(|s| {
                        let name = &s.file.name;
                        let stem = name.strip_suffix(".mid").or(name.strip_suffix(".midi")).unwrap_or(name);
                        crate::library::pretty_title_of(stem).0
                    })
                    .unwrap_or_default();

                let r = widgets::card(ui, ("main_continue", 0), WIDE_W, 64.0, |ui, t| {
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
                        .font_family(theme::FONT_DISPLAY)
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
                            if widgets::button(("main_continue_play", 0), locals::menu::play())
                                .primary()
                                .size(96.0, 36.0)
                                .font_size(theme::FONT_S)
                                .build(ui)
                                .clicked
                            {
                                act = Some(Act::PlayLast);
                            }
                        });
                });
                if r.clicked {
                    act = Some(Act::PlayLast);
                }
            });
            // Avanço fora do bloco com âmbito (regra do A)
            y += 22.0 + theme::SPACE_XS + 64.0 + drop;
        }

        // ── Ações secundárias: Abrir ficheiro · Tocar livre ────────────
        y += theme::SPACE_XL;
        let act_w = 200.0;
        let act_gap = theme::SPACE_M;
        let row_w = act_w * 2.0 + act_gap;
        nuon::translate().x(cx - row_w / 2.0).y(y).build(ui, |ui| {
            if widgets::button(("main_open", 0), locals::menu::open_file())
                .icon(FOLDER_ICON)
                .size(act_w, theme::BUTTON_H)
                .build(ui)
                .clicked
            {
                act = Some(Act::OpenFile);
            }
            nuon::translate().x(act_w + act_gap).add_to_current(ui);
            if widgets::button(("main_free", 0), locals::menu::freeplay())
                .icon(icons::balloon_icon())
                .size(act_w, theme::BUTTON_H)
                .build(ui)
                .clicked
            {
                act = Some(Act::Freeplay);
            }
        });
        y += theme::BUTTON_H + theme::SPACE_XL;

        // ── Recentes (até 5, com autor) ────────────────────────────────
        let recent: Vec<_> = ctx.library.recent.iter().take(5).collect();
        if !recent.is_empty() {
            nuon::translate().x(cx - WIDE_W / 2.0).y(y).build(ui, |ui| {
                widgets::eyebrow(ui, WIDE_W, &locals::library::recent());
            });
            y += 18.0 + theme::SPACE_S;

            for (i, path) in recent.iter().enumerate() {
                let (title, composer) = Library::pretty_title(path);
                let sub = composer.unwrap_or_default();
                nuon::translate().x(cx - WIDE_W / 2.0).y(y).build(ui, |ui| {
                    let mut row = widgets::list_row(("main_recent", i), WIDE_W, &title)
                        .height(44.0);
                    if !sub.is_empty() {
                        row = row.subtitle(&sub);
                    }
                    let r = row.build(ui);
                    if r.clicked {
                        act = Some(Act::OpenSong((*path).clone()));
                    }
                });
                y += 44.0 + theme::SPACE_XS;
            }
        }

        // ── Sair (ghost, canto do conteúdo) ────────────────────────────
        nuon::translate()
            .x(nav.content_x + content_w - theme::SPACE_XL - 110.0)
            .y(win_h - theme::SPACE_XL - 44.0)
            .build(ui, |ui| {
                if widgets::button(("main_exit", 0), locals::menu::exit())
                    .ghost()
                    .size(110.0, 44.0)
                    .build(ui)
                    .clicked
                {
                    act = Some(Act::Exit);
                }
            });

        self.apply_nav_actions(ctx, nav_act);

        // ── Aplicar ação ───────────────────────────────────────────────
        let Some(act) = act else { return };
        match act {
            Act::PlayLast => state::play(&self.state, ctx),
            Act::OpenSong(path) => self.state.open_song(ctx, path),
            Act::OpenFile => self.futures.push(open_midi_file_picker(&mut self.state, ctx)),
            Act::Freeplay => state::freeplay(&self.state, ctx),
            Act::Exit => self.state.go_back(),
        }
    }
}
