//! Navegação lateral partilhada (redesign "sala de concerto", tarefa R1).
//!
//! As páginas chamam `side_nav(...)` no início do desenho e deslocam o
//! conteúdo para `NavLayout::content_x`. As ações devolvidas aplicam-se com
//! `apply_actions` no fim da página (fora do desenho, padrão `enum Act`).

use std::hash::Hash;

use nuon::TextAlign;

use super::Page;
use crate::{
    context::Context,
    icons,
    locals,
    scene::menu_scene::library_page::play_path,
    theme,
    widgets,
};

/// Glifos bootstrap-icons v1.11.3 (a mover para `icons.rs`)
pub const HOME_ICON: &str = "\u{F425}";
pub const LIBRARY_ICON: &str = "\u{F49F}";
pub const MIC_ICON: &str = "\u{F48D}";
pub const CHART_ICON: &str = "\u{F17B}";
pub const SLIDERS_ICON: &str = "\u{F56B}";
pub const PLAY_CIRCLE_ICON: &str = "\u{F4F2}";

const NAV_W: f32 = 232.0;
const NAV_W_ICONS: f32 = 64.0;
const ITEM_H: f32 = 44.0;
const PAD: f32 = 10.0;

/// Ações recolhidas pela navegação; a página aplica-as no fim.
#[derive(Default, Debug, Clone, Copy)]
pub struct NavActions {
    pub go_main: bool,
    pub go_library: bool,
    pub go_recordings: bool,
    pub go_settings: bool,
    pub play_queue: bool,
}

/// Geometria da navegação: onde e com que largura a página desenha o conteúdo.
#[derive(Debug, Clone, Copy)]
pub struct NavLayout {
    pub w: f32,
    pub content_x: f32,
    pub content_w: f32,
}

/// Dourado a 12 % — fundo do item ativo
fn active_bg() -> nuon::Color {
    theme::accent().mix(theme::background(), 0.88)
}

impl super::MenuScene {
    /// Desenha a navegação lateral e devolve as ações + a área de conteúdo.
    pub fn side_nav(&self, ctx: &mut Context, ui: &mut nuon::Ui) -> (NavLayout, NavActions) {
        let win_w = ctx.window_state.logical_size.width;
        let win_h = ctx.window_state.logical_size.height;
        let icons_only = win_w < 900.0;
        let w = if icons_only { NAV_W_ICONS } else { NAV_W };
        let current = self.state.current();
        let mut act = NavActions::default();

        // Painel de fundo, com borda à direita
        nuon::quad()
            .size(w, win_h)
            .color(theme::surface())
            .border_color(theme::border())
            .border_width(1.0)
            .build(ui);

        // Logótipo
        nuon::translate().y(theme::SPACE_L).build(ui, |ui| {
            nuon::label()
                .size(w, 36.0)
                .font_size(26.0)
                .font_family(theme::FONT_DISPLAY)
                .bold(true)
                .color(theme::text())
                .text_justify(if icons_only {
                    TextAlign::Center
                } else {
                    TextAlign::Start
                })
                .x(if icons_only { 0.0 } else { theme::SPACE_L })
                .text(if icons_only { "N" } else { "Neothasia" })
                .build(ui);
        });
        let mut y = 36.0 + theme::SPACE_L * 2.0;

        // Itens; Estatísticas fica desativada até existir Page::Stats (B11)
        let items: [(Option<Page>, &str, &str, bool); 5] = [
            (Some(Page::Main), HOME_ICON, "Início", true),
            (Some(Page::Library), LIBRARY_ICON, "Biblioteca", true),
            (
                Some(Page::Recordings),
                MIC_ICON,
                "Gravações",
                ctx.config.last_opened_song().is_some(),
            ),
            (None, CHART_ICON, "Estatísticas", false),
            (Some(Page::Settings), SLIDERS_ICON, "Definições", true),
        ];

        for (target, icon, label, enabled) in items {
            let selected = target.is_some_and(|t| *current == t);
            let ev = nuon::click_area(("nav_item", label))
                .pos(PAD, y)
                .size(w - PAD * 2.0, ITEM_H)
                .build(ui);

            nuon::quad()
                .pos(PAD, y)
                .size(w - PAD * 2.0, ITEM_H)
                .color(if selected {
                    active_bg()
                } else if ev.is_hovered() && enabled {
                    theme::surface_hover()
                } else {
                    nuon::Color::new_u8(0, 0, 0, 0.0)
                })
                .border_radius([theme::RADIUS_M; 4])
                .build(ui);

            let color = if !enabled || (!selected && ev.is_hovered()) {
                theme::text_muted()
            } else if selected {
                theme::accent()
            } else {
                theme::text()
            };

            nuon::label()
                .pos(if icons_only { 0.0 } else { PAD + 10.0 }, y)
                .size(if icons_only { w } else { 34.0 }, ITEM_H)
                .font_size(20.0)
                .color(color)
                .icon(icon)
                .build(ui);

            if !icons_only {
                nuon::label()
                    .pos(PAD + 48.0, y)
                    .size(w - PAD * 2.0 - 48.0, ITEM_H)
                    .font_size(theme::FONT_M)
                    .color(if enabled { color } else { theme::text_faint() })
                    .text_justify(TextAlign::Start)
                    .text(label.to_string())
                    .build(ui);
            }

            if ev.is_clicked() && enabled {
                match target {
                    Some(Page::Main) => act.go_main = true,
                    Some(Page::Library) => act.go_library = true,
                    Some(Page::Recordings) => act.go_recordings = true,
                    Some(Page::Settings) => act.go_settings = true,
                    _ => {}
                }
            }

            y += ITEM_H + theme::SPACE_XS;
        }

        // ── Bloco "Playlist" no fundo ────────────────────────────────
        let block_h = 74.0;
        let block_w = w - PAD * 2.0;
        let block_y = win_h - block_h - PAD;
        let queue_len = ctx.library.queue.len();

        nuon::translate().y(block_y).build(ui, |ui| {
            nuon::quad()
                .x(PAD)
                .size(block_w, block_h)
                .color(theme::surface_sunken())
                .border_color(theme::border())
                .border_width(1.0)
                .border_radius([theme::RADIUS_M; 4])
                .build(ui);

            if !icons_only {
                nuon::translate().x(PAD).build(ui, |ui| {
                    widgets::eyebrow(ui, block_w - PAD * 2.0, "Playlist");
                });
            }

            let info = match ctx.library.position_label() {
                Some(label) => label,
                None => format!(
                    "{} {}",
                    queue_len,
                    if queue_len == 1 {
                        locals::library::one_song()
                    } else {
                        locals::library::songs_count()
                    }
                ),
            };
            nuon::label()
                .x(PAD + theme::SPACE_M)
                .y(if icons_only { 0.0 } else { 18.0 })
                .size(block_w - theme::SPACE_M * 3.0 - 36.0, 34.0)
                .font_size(theme::FONT_S)
                .color(theme::text_muted())
                .text_justify(TextAlign::Start)
                .text(info)
                .build(ui);

            if queue_len > 0 {
                nuon::translate()
                    .x(block_w - 36.0 - theme::SPACE_M)
                    .y(if icons_only { 0.0 } else { 18.0 })
                    .build(ui, |ui| {
                        if widgets::button(("nav_play_queue", 0), "")
                            .icon(icons::play_icon())
                            .primary()
                            .size(36.0, 36.0)
                            .build(ui)
                            .clicked
                        {
                            act.play_queue = true;
                        }
                    });
            }
        });

        (
            NavLayout {
                w,
                content_x: w,
                content_w: win_w - w,
            },
            act,
        )
    }

    /// Aplica as ações da navegação (chamar no fim da página).
    pub fn apply_nav_actions(&mut self, ctx: &mut Context, act: NavActions) {
        if act.go_main {
            // go_home() ainda não existe no UiState (campo privado); volta
            // atrás, com limite, até ao Início. A pedir ao A para acrescentar.
            for _ in 0..8 {
                if self.state.current() == &Page::Main {
                    break;
                }
                self.state.go_back();
            }
        }
        if act.go_library {
            self.state.go_to(Page::Library);
        }
        if act.go_recordings
            && let Some(path) = ctx.config.last_opened_song().cloned()
        {
            self.state.open_recordings(ctx, path);
        }
        if act.go_settings {
            self.state.go_to(Page::Settings);
        }
        if act.play_queue
            && let Some(path) = ctx.library.queue.first().cloned()
        {
            play_path(&self.state, ctx, path, Some(0));
        }
    }
}
