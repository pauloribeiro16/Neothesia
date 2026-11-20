//! Página "Biblioteca": lista de músicas pesquisável + playlist.
//!
//! Layout (ver `.agents/TAREFAS.md` › B6): cabeçalho com pesquisa, painel
//! esquerdo com as músicas, painel direito com a playlist, barra inferior com
//! ←, pastas e ▶ Tocar playlist. Teclado: ↑/↓ cursor, Enter toca, Tab junta
//! à playlist, Esc limpa a pesquisa ou volta.

use std::{hash::Hash, path::PathBuf};

use midi_file::MidiFile;

use super::{MsgFn, UiState, on_async, state};
use crate::{
    NeothesiaEvent, context::Context, icons, library::Library, locals, song::Song, theme,
    utils::BoxFuture, widgets,
};

/// Glifos bootstrap-icons ainda não em `crate::icons` (a pedir ao A)
const STAR_ICON: &str = "\u{F588}";
const STAR_FILL_ICON: &str = "\u{F586}";
const ARROW_UP_ICON: &str = "\u{F148}";
const ARROW_DOWN_ICON: &str = "\u{F128}";

const TRAILING_W: f32 = 36.0 * 3.0 + 8.0 * 2.0;
const CHIP_H: f32 = 32.0;
const BOTTOM_BAR_H: f32 = 60.0;
const HEADER_H: f32 = 56.0;

enum Act {
    PlayNow(PathBuf),
    PlayQueueFrom(usize),
    Enqueue(PathBuf),
    ToggleFavorite(PathBuf),
    QueueUp(usize),
    QueueDown(usize),
    QueueRemove(usize),
    ClearQueue,
    RemoveFolder(usize),
    AddFolder,
    Rescan,
}

/// Nome da pasta que contém `path` (para o subtítulo das linhas)
fn folder_name(path: &std::path::Path) -> String {
    path.parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// Largura estimada de um chip de pasta
fn chip_w(name: &str) -> f32 {
    (name.chars().count() as f32 * 8.0 + 44.0).clamp(80.0, 180.0)
}

/// Botão pequeno de ícone com tooltip (o `widgets::icon_button` ainda não
/// expõe tooltip; pedido ao A para o mover para `widgets.rs`)
fn icon_btn(
    ui: &mut nuon::Ui,
    id: impl Hash,
    icon: &str,
    size: f32,
    tip: impl Into<String>,
) -> widgets::Response {
    widgets::button(id, "")
        .icon(icon)
        .size(size, size)
        .ghost()
        .tooltip(tip)
        .build(ui)
}

pub fn play_path(
    state: &UiState,
    ctx: &mut Context,
    path: PathBuf,
    queue_pos: Option<usize>,
) -> bool {
    let midi = match MidiFile::new(&path) {
        Ok(m) => m,
        Err(e) => {
            log::error!("Failed to open {path:?}: {e}");
            return false;
        }
    };
    let song = Song::new(midi);

    ctx.library.mark_played(&path);
    ctx.config.set_last_opened_song(Some(path));
    ctx.library.queue_pos = queue_pos;

    state::connect_io(state, ctx);
    ctx.proxy.send_event(NeothesiaEvent::Play(song)).ok();
    true
}

fn pick_folder_fut() -> BoxFuture<MsgFn> {
    on_async(
        async {
            rfd::AsyncFileDialog::new()
                .pick_folder()
                .await
                .map(|f| f.path().to_path_buf())
        },
        |res: Option<PathBuf>, _data: &mut UiState, ctx: &mut Context| {
            if let Some(folder) = res {
                ctx.library.add_folder(folder);
            }
        },
    )
}

impl super::MenuScene {
    /// Teclado da página Biblioteca (chamado por `MenuScene::window_event`).
    pub fn library_page_window_event(
        &mut self,
        ctx: &mut Context,
        event: &winit::event::WindowEvent,
    ) {
        use winit::{
            event::WindowEvent,
            keyboard::{Key, NamedKey},
        };
        let WindowEvent::KeyboardInput { event: key, .. } = event else {
            return;
        };
        if !key.state.is_pressed() {
            return;
        }

        let visible = self.visible_songs(ctx);

        if key.logical_key == Key::Named(NamedKey::Escape) {
            if self.state.search.is_empty() {
                self.state.go_back();
            } else {
                self.state.search.clear();
            }
        } else if key.logical_key == Key::Named(NamedKey::Backspace) {
            self.state.search.pop();
        } else if key.logical_key == Key::Named(NamedKey::ArrowUp) {
            self.state.lib_cursor = self.state.lib_cursor.saturating_sub(1);
        } else if key.logical_key == Key::Named(NamedKey::ArrowDown) {
            self.state.lib_cursor = (self.state.lib_cursor + 1).min(visible.len().saturating_sub(1));
        } else if key.logical_key == Key::Named(NamedKey::Enter) {
            if let Some(song) = visible.get(self.state.lib_cursor) {
                let path = song.path.clone();
                play_path(&self.state, ctx, path, None);
            }
        } else if key.logical_key == Key::Named(NamedKey::Tab) {
            if let Some(song) = visible.get(self.state.lib_cursor) {
                ctx.library.enqueue(song.path.clone());
            }
        } else if let Some(text) = &key.text {
            self.state
                .search
                .extend(text.chars().filter(|c| !c.is_control()));
        }
    }
}

impl super::MenuScene {
    /// Índices das músicas que correspondem à pesquisa atual
    fn visible_songs<'a>(&self, ctx: &'a Context) -> Vec<&'a crate::library::LibSong> {
        let query = self.state.search.to_lowercase();
        ctx.library
            .songs
            .iter()
            .filter(|s| s.matches(&query))
            .collect()
    }

    pub fn library_page_ui(&mut self, ctx: &mut Context, ui: &mut nuon::Ui) {
        let win_w = ctx.window_state.logical_size.width;
        let win_h = ctx.window_state.logical_size.height;

        let visible = self.visible_songs(ctx);
        let mut act: Option<Act> = None;

        // ── Cabeçalho: título + pesquisa ────────────────────────────
        let search_w = 360.0_f32.min(win_w - 320.0).max(160.0);
        nuon::translate().x(theme::SPACE_XL).build(ui, |ui| {
            nuon::label()
                .y(theme::SPACE_S)
                .size(win_w - theme::SPACE_XL * 2.0, 44.0)
                .font_size(theme::FONT_XXL)
                .color(theme::text())
                .bold(true)
                .text_justify(nuon::TextAlign::Start)
                .text(locals::library::title())
                .build(ui);
        });

        nuon::translate()
            .x(win_w - theme::SPACE_XL - search_w)
            .y(nuon::center_y(HEADER_H, theme::BUTTON_H))
            .build(ui, |ui| {
                if widgets::search_box(
                    ui,
                    ("lib_search", 0),
                    search_w,
                    &self.state.search,
                    &locals::library::search_placeholder(),
                ) {
                    self.state.search.clear();
                }
            });

        // ── Painéis ─────────────────────────────────────────────────
        let panels_y = HEADER_H + theme::SPACE_S;
        let panels_h = (win_h - panels_y - BOTTOM_BAR_H - theme::SPACE_S * 2.0).max(100.0);
        let avail_w = (win_w - theme::SPACE_XL * 2.0).max(200.0);
        let panel_gap = theme::SPACE_M;
        let left_w = ((avail_w - panel_gap) * 0.62).max(260.0);
        let right_w = (avail_w - panel_gap - left_w).max(160.0);

        // Painel esquerdo: músicas
        nuon::translate()
            .x(theme::SPACE_XL)
            .y(panels_y)
            .build(ui, |ui| {
                theme::panel(ui, left_w, panels_h);
                nuon::translate()
                    .x(theme::SPACE_L)
                    .y(theme::SPACE_L)
                    .build(ui, |ui| {
                        let inner_w = left_w - theme::SPACE_L * 2.0;
                        widgets::section_title(
                            ui,
                            inner_w,
                            &locals::library::musics(),
                            Some(visible.len()),
                        );

                        let rows_h =
                            (panels_h - theme::SPACE_L * 2.0 - 28.0 - theme::SPACE_S).max(40.0);
                        let cursor = self.state.lib_cursor.min(visible.len().saturating_sub(1));
                        let playing_path = ctx
                            .library
                            .queue_pos
                            .and_then(|i| ctx.library.queue.get(i))
                            .cloned();

                        nuon::translate()
                            .y(28.0 + theme::SPACE_S)
                            .add_to_current(ui);
                        nuon::scroll("library_scroll")
                            .scissor_size(inner_w, rows_h)
                            .build(ui, |ui| {
                                if visible.is_empty() {
                                    let hint = if self.state.search.is_empty()
                                        || !ctx.library.folders.is_empty()
                                    {
                                        Some(locals::library::empty_hint())
                                    } else {
                                        None
                                    };

                                    match hint {
                                        Some(text) => {
                                            widgets::hint(ui, inner_w, 40.0, &text);
                                            nuon::translate()
                                                .y(theme::SPACE_M)
                                                .add_to_current(ui);
                                            if widgets::button(
                                                ("lib_add", 0),
                                                locals::library::add_folder_action(),
                                            )
                                            .primary()
                                            .build(ui)
                                            .clicked
                                            {
                                                act = Some(Act::AddFolder);
                                            }
                                        }
                                        None => widgets::hint(
                                            ui,
                                            inner_w,
                                            40.0,
                                            &locals::library::no_results(),
                                        ),
                                    }
                                    return;
                                }

                                let row_w = inner_w - 14.0; // espaço para a scrollbar do scroll
                                for (pos, song) in visible.iter().enumerate() {
                                    let sub = folder_name(&song.path);
                                    let is_playing = playing_path.as_ref() == Some(&song.path);
                                    let is_fav = ctx.library.is_favorite(&song.path);

                                    widgets::list_row(("lib_row", pos), row_w, &song.title)
                                    .subtitle(&sub)
                                    .selected(pos == cursor)
                                    .active(is_playing)
                                    .trailing(TRAILING_W)
                                    .build(ui);

                                    // Trailing: [▶] [★] [+]
                                    nuon::translate().x(row_w - TRAILING_W).build(ui, |ui| {
                                        if icon_btn(
                                            ui,
                                            ("lib_play", pos),
                                            icons::play_icon(),
                                            36.0,
                                            locals::library::play_now(),
                                        )
                                        .clicked
                                        {
                                            act = Some(Act::PlayNow(song.path.clone()));
                                        }
                                        nuon::translate()
                                            .x(36.0 + theme::SPACE_S)
                                            .add_to_current(ui);

                                        let star = if is_fav {
                                            STAR_FILL_ICON
                                        } else {
                                            STAR_ICON
                                        };
                                        if icon_btn(
                                            ui,
                                            ("lib_fav", pos),
                                            star,
                                            36.0,
                                            if is_fav {
                                                locals::library::favorite_remove()
                                            } else {
                                                locals::library::favorite_add()
                                            },
                                        )
                                        .clicked
                                        {
                                            act = Some(Act::ToggleFavorite(song.path.clone()));
                                        }
                                        nuon::translate()
                                            .x(36.0 + theme::SPACE_S)
                                            .add_to_current(ui);

                                        if icon_btn(
                                            ui,
                                            ("lib_add", pos),
                                            icons::plus_icon(),
                                            36.0,
                                            locals::library::enqueue(),
                                        )
                                        .clicked
                                        {
                                            act = Some(Act::Enqueue(song.path.clone()));
                                        }
                                    });

                                    nuon::translate()
                                        .y(theme::ROW_H + theme::SPACE_XS)
                                        .add_to_current(ui);
                                }
                            });
                    });
            });

        // Painel direito: playlist
        nuon::translate()
            .x(theme::SPACE_XL + left_w + panel_gap)
            .y(panels_y)
            .build(ui, |ui| {
                theme::panel(ui, right_w, panels_h);
                nuon::translate()
                    .x(theme::SPACE_L)
                    .y(theme::SPACE_L)
                    .build(ui, |ui| {
                        let inner_w = right_w - theme::SPACE_L * 2.0;
                        widgets::section_title(
                            ui,
                            inner_w,
                            &locals::library::playlist(),
                            Some(ctx.library.queue.len()),
                        );

                        // "Limpar" no canto do título
                        nuon::translate().x(inner_w - 84.0).build(ui, |ui| {
                            if widgets::button(("q_clear", 0), locals::library::clear())
                                .size(84.0, 28.0)
                                .ghost()
                                .font_size(theme::FONT_S)
                                .build(ui)
                                .clicked
                            {
                                act = Some(Act::ClearQueue);
                            }
                        });

                        let rows_h =
                            (panels_h - theme::SPACE_L * 2.0 - 28.0 - theme::SPACE_S).max(40.0);
                        nuon::translate().y(28.0 + theme::SPACE_S).add_to_current(ui);
                        nuon::scroll("queue_scroll")
                            .scissor_size(inner_w, rows_h)
                            .build(ui, |ui| {
                                if ctx.library.queue.is_empty() {
                                    widgets::hint(
                                        ui,
                                        inner_w,
                                        60.0,
                                        &locals::library::queue_empty_hint(),
                                    );
                                    return;
                                }

                                let row_w = inner_w - 14.0; // espaço para a scrollbar do scroll
                                for (i, path) in ctx.library.queue.iter().enumerate() {
                                    let title = Library::display_name(path);
                                    widgets::list_row(("q_row", i), row_w, &title)
                                        .leading(format!("{}", i + 1))
                                        .active(ctx.library.queue_pos == Some(i))
                                        .trailing(TRAILING_W)
                                        .build(ui);

                                    // Trailing: [↑] [↓] [×]
                                    nuon::translate().x(row_w - TRAILING_W).build(ui, |ui| {
                                        if icon_btn(
                                            ui,
                                            ("q_up", i),
                                            ARROW_UP_ICON,
                                            36.0,
                                            locals::library::queue_up(),
                                        )
                                        .clicked
                                        {
                                            act = Some(Act::QueueUp(i));
                                        }
                                        nuon::translate()
                                            .x(36.0 + theme::SPACE_S)
                                            .add_to_current(ui);

                                        if icon_btn(
                                            ui,
                                            ("q_dn", i),
                                            ARROW_DOWN_ICON,
                                            36.0,
                                            locals::library::queue_down(),
                                        )
                                        .clicked
                                        {
                                            act = Some(Act::QueueDown(i));
                                        }
                                        nuon::translate()
                                            .x(36.0 + theme::SPACE_S)
                                            .add_to_current(ui);

                                        if icon_btn(
                                            ui,
                                            ("q_rm", i),
                                            icons::trash_icon(),
                                            36.0,
                                            locals::library::remove(),
                                        )
                                        .clicked
                                        {
                                            act = Some(Act::QueueRemove(i));
                                        }
                                    });

                                    nuon::translate()
                                        .y(theme::ROW_H + theme::SPACE_XS)
                                        .add_to_current(ui);
                                }
                            });
                    });
            });

        // ── Barra inferior ──────────────────────────────────────────
        nuon::translate().y(win_h - BOTTOM_BAR_H).build(ui, |ui| {
            let btn_h = 44.0;
            let bar_y = nuon::center_y(BOTTOM_BAR_H, btn_h);

            // ← voltar
            nuon::translate().x(theme::SPACE_L).y(bar_y).build(ui, |ui| {
                if widgets::button(("lib_back", 0), "")
                    .icon(icons::left_arrow_icon())
                    .size(btn_h, btn_h)
                    .build(ui)
                    .clicked
                {
                    self.state.go_back();
                }
            });

            // Pastas ligadas (chips com ×) + "+ Pasta" + Atualizar
            let mut x = theme::SPACE_L + btn_h + theme::SPACE_M;
            nuon::translate().x(x).build(ui, |ui| {
                let chip_y = nuon::center_y(BOTTOM_BAR_H, CHIP_H);
                for (i, folder) in ctx.library.folders.iter().enumerate() {
                    let name = folder
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| folder.display().to_string());
                    let w = chip_w(&name);
                    if x + w + btn_h * 2.0 + theme::SPACE_M * 3.0 > win_w - 220.0 {
                        break;
                    }
                    nuon::translate().x(x).y(chip_y).build(ui, |ui| {
                        if widgets::button(("lib_folder", i), format!("× {name}"))
                            .size(w, CHIP_H)
                            .font_size(theme::FONT_S)
                            .tooltip(locals::library::remove_folder())
                            .build(ui)
                            .clicked
                        {
                            act = Some(Act::RemoveFolder(i));
                        }
                    });
                    x += w + theme::SPACE_XS;
                }

                x += theme::SPACE_S;
                nuon::translate().x(x).y(bar_y).build(ui, |ui| {
                    if widgets::button(("lib_add_folder", 0), locals::library::add_folder())
                        .size(120.0, btn_h)
                        .build(ui)
                        .clicked
                    {
                        act = Some(Act::AddFolder);
                    }
                });
                x += 120.0 + theme::SPACE_XS;

                nuon::translate().x(x).y(bar_y).build(ui, |ui| {
                    if icon_btn(
                        ui,
                        ("lib_rescan", 0),
                        icons::repeat_icon(),
                        btn_h,
                        locals::library::refresh(),
                    )
                    .clicked
                    {
                        act = Some(Act::Rescan);
                    }
                });
            });

            // ▶ Tocar playlist
            if !ctx.library.queue.is_empty() {
                nuon::translate()
                    .x(win_w - theme::SPACE_L - 200.0)
                    .y(bar_y)
                    .build(ui, |ui| {
                        if widgets::button(("q_play", 0), locals::library::play_playlist())
                            .icon(icons::play_icon())
                            .size(200.0, btn_h)
                            .primary()
                            .build(ui)
                            .clicked
                        {
                            act = Some(Act::PlayQueueFrom(0));
                        }
                    });
            }
        });

        // ── Aplicar ação ────────────────────────────────────────────
        let Some(act) = act else { return };
        match act {
            Act::PlayNow(path) => {
                play_path(&self.state, ctx, path, None);
            }
            Act::PlayQueueFrom(i) => {
                if let Some(path) = ctx.library.queue.get(i).cloned() {
                    play_path(&self.state, ctx, path, Some(i));
                }
            }
            Act::Enqueue(path) => ctx.library.enqueue(path),
            Act::ToggleFavorite(path) => ctx.library.toggle_favorite(&path),
            Act::QueueUp(i) => ctx.library.move_in_queue(i, true),
            Act::QueueDown(i) => ctx.library.move_in_queue(i, false),
            Act::QueueRemove(i) => ctx.library.remove_from_queue(i),
            Act::ClearQueue => ctx.library.clear_queue(),
            Act::RemoveFolder(i) => ctx.library.remove_folder(i),
            Act::AddFolder => self.futures.push(pick_folder_fut()),
            Act::Rescan => ctx.library.rescan(),
        }
    }
}
