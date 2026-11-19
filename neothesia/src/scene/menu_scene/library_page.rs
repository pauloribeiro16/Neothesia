//! Página "Biblioteca": lista de músicas pesquisável + playlist.

use std::{hash::Hash, path::PathBuf};

use midi_file::MidiFile;
use nuon::Color;

use super::{MsgFn, UiState, neo_btn, neo_btn_icon, on_async, state};
use crate::{
    NeothesiaEvent, context::Context, icons, library::Library, song::Song, utils::BoxFuture,
};

const ROW_H: f32 = 44.0;
const ROW_GAP: f32 = 6.0;
const SMALL_BTN: f32 = 40.0;

enum Act {
    PlayNow(PathBuf),
    PlayQueueFrom(usize),
    Enqueue(PathBuf),
    EnqueueAllVisible,
    QueueUp(usize),
    QueueDown(usize),
    QueueRemove(usize),
    ClearQueue,
    RemoveFolder(usize),
    AddFolder,
    Rescan,
}

fn row_id(tag: &str, i: usize) -> nuon::Id {
    nuon::Id::hash_with(|h| {
        tag.hash(h);
        i.hash(h);
    })
}

fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_owned()
    } else {
        let cut: String = s.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{cut}…")
    }
}

/// Botão pequeno de texto (usado em listas, onde `neo_btn` seria demasiado grande)
fn small_btn(ui: &mut nuon::Ui, id: nuon::Id, w: f32, h: f32, text: &str) -> bool {
    let ev = nuon::click_area(id).size(w, h).build(ui);
    let bg = if ev.is_hovered() || ev.is_pressed() {
        Color::new_u8(56, 145, 255, 0.9)
    } else {
        Color::new_u8(60, 50, 90, 0.8)
    };
    nuon::quad()
        .size(w, h)
        .color(bg)
        .border_radius([7.0; 4])
        .build(ui);
    nuon::label()
        .size(w, h)
        .font_size(18.0)
        .text(text)
        .text_justify(nuon::TextAlign::Center)
        .build(ui);
    ev.is_clicked()
}

/// Linha clicável com título; devolve `true` se clicada.
fn title_row(
    ui: &mut nuon::Ui,
    id: nuon::Id,
    w: f32,
    title: &str,
    highlighted: bool,
) -> bool {
    let ev = nuon::click_area(id).size(w, ROW_H).build(ui);
    let bg = if ev.is_hovered() || ev.is_pressed() {
        Color::new_u8(30, 30, 40, 0.85)
    } else if highlighted {
        Color::new_u8(60, 40, 110, 0.8)
    } else {
        Color::new_u8(17, 17, 17, 0.6)
    };
    nuon::quad()
        .size(w, ROW_H)
        .color(bg)
        .border_radius([7.0; 4])
        .build(ui);
    if highlighted {
        nuon::quad()
            .size(5.0, ROW_H)
            .color(Color::new_u8(160, 81, 255, 1.0))
            .border_radius([7.0, 0.0, 0.0, 7.0])
            .build(ui);
    }
    nuon::label()
        .x(14.0)
        .size(w - 14.0, ROW_H)
        .font_size(18.0)
        .text(truncate(title, ((w - 24.0) / 9.5).max(8.0) as usize))
        .build(ui);
    ev.is_clicked()
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
        _ctx: &mut Context,
        event: &winit::event::WindowEvent,
    ) {
        use winit::{event::WindowEvent, keyboard::{Key, NamedKey}};
        if let WindowEvent::KeyboardInput { event: key, .. } = event
            && key.state.is_pressed()
        {
            if key.logical_key == Key::Named(NamedKey::Escape) {
                if self.state.search.is_empty() {
                    self.state.go_back();
                } else {
                    self.state.search.clear();
                }
            } else if key.logical_key == Key::Named(NamedKey::Backspace) {
                self.state.search.pop();
            } else if let Some(text) = &key.text {
                self.state
                    .search
                    .extend(text.chars().filter(|c| !c.is_control()));
            }
        }
    }

    pub fn library_page_ui(&mut self, ctx: &mut Context, ui: &mut nuon::Ui) {
        let win_w = ctx.window_state.logical_size.width;
        let win_h = ctx.window_state.logical_size.height;

        let bottom_bar_h = 60.0;
        let margin = 14.0;
        let header_h = 56.0;

        let query_lower = self.state.search.to_lowercase();
        let mut act: Option<Act> = None;

        // ── Cabeçalho ───────────────────────────────────────────────
        let visible: Vec<usize> = ctx
            .library
            .songs
            .iter()
            .enumerate()
            .filter(|(_, s)| s.matches(&query_lower))
            .map(|(i, _)| i)
            .collect();

        let header = if ctx.library.folders.is_empty() {
            "Biblioteca vazia — adiciona uma pasta com músicas (.mid) ou arrasta-a para aqui"
                .to_owned()
        } else {
            format!(
                "Biblioteca · {} de {} músicas · {} pasta(s)",
                visible.len(),
                ctx.library.songs.len(),
                ctx.library.folders.len()
            )
        };

        nuon::label()
            .pos(margin, 8.0)
            .size(win_w - margin * 2.0, 24.0)
            .font_size(18.0)
            .text(header)
            .build(ui);

        let search_text = if self.state.search.is_empty() {
            "Escreve para pesquisar…".to_owned()
        } else {
            format!("Pesquisa: {}_", self.state.search)
        };
        nuon::label()
            .pos(margin, 32.0)
            .size(win_w - margin * 2.0, 22.0)
            .font_size(16.0)
            .color(if self.state.search.is_empty() {
                Color::new_u8(150, 150, 150, 1.0)
            } else {
                Color::WHITE
            })
            .text(search_text)
            .build(ui);

        // ── Colunas ─────────────────────────────────────────────────
        let list_top = header_h + 4.0;
        let list_h = (win_h - list_top - bottom_bar_h - margin * 2.0).max(60.0);
        let total_w = win_w - margin * 3.0;
        let left_w = (total_w * 0.58).max(200.0);
        let right_w = (total_w - left_w).max(160.0);

        // Biblioteca (esquerda)
        nuon::translate().x(margin).y(list_top).build(ui, |ui| {
            nuon::scroll("library_scroll")
                .scissor_size(left_w, list_h)
                .build(ui, |ui| {
                    let row_w = left_w - 16.0;
                    for &i in &visible {
                        let song = &ctx.library.songs[i];
                        let in_queue = ctx.library.queue.contains(&song.path);
                        nuon::translate().build(ui, |ui| {
                            if title_row(ui, row_id("lib_play", i), row_w - SMALL_BTN - 8.0, &song.title, in_queue) {
                                act = Some(Act::PlayNow(song.path.clone()));
                            }
                            nuon::translate()
                                .x(row_w - SMALL_BTN)
                                .y((ROW_H - SMALL_BTN) / 2.0)
                                .build(ui, |ui| {
                                    if small_btn(ui, row_id("lib_add", i), SMALL_BTN, SMALL_BTN, "+") {
                                        act = Some(Act::Enqueue(song.path.clone()));
                                    }
                                });
                        });
                        nuon::translate().y(ROW_H + ROW_GAP).add_to_current(ui);
                    }
                });
        });

        // Playlist (direita)
        nuon::translate()
            .x(margin * 2.0 + left_w)
            .y(list_top)
            .build(ui, |ui| {
                let queue_head_h = 40.0;
                nuon::label()
                    .size(right_w, 24.0)
                    .font_size(18.0)
                    .text(format!("Playlist ({})", ctx.library.queue.len()))
                    .build(ui);

                let head_btn_w = 110.0;
                nuon::translate()
                    .x(right_w - head_btn_w * 2.0 - 8.0)
                    .build(ui, |ui| {
                        if small_btn(ui, row_id("q_all", 0), head_btn_w, 30.0, "+ Visíveis") {
                            act = Some(Act::EnqueueAllVisible);
                        }
                        nuon::translate().x(head_btn_w + 8.0).add_to_current(ui);
                        if small_btn(ui, row_id("q_clear", 0), head_btn_w, 30.0, "Limpar") {
                            act = Some(Act::ClearQueue);
                        }
                    });

                nuon::translate().y(queue_head_h).build(ui, |ui| {
                    nuon::scroll("queue_scroll")
                        .scissor_size(right_w, (list_h - queue_head_h).max(40.0))
                        .build(ui, |ui| {
                            if ctx.library.queue.is_empty() {
                                nuon::label()
                                    .size(right_w, 60.0)
                                    .font_size(16.0)
                                    .color(Color::new_u8(150, 150, 150, 1.0))
                                    .text("Usa o botão + para juntar músicas")
                                    .build(ui);
                            }

                            let row_w = right_w - 16.0;
                            let btns_w = SMALL_BTN * 3.0 + 12.0;
                            for (i, path) in ctx.library.queue.iter().enumerate() {
                                let playing = ctx.library.queue_pos == Some(i);
                                let title = format!("{}. {}", i + 1, Library::display_name(path));
                                nuon::translate().build(ui, |ui| {
                                    if title_row(ui, row_id("q_row", i), row_w - btns_w - 6.0, &title, playing) {
                                        act = Some(Act::PlayQueueFrom(i));
                                    }
                                    nuon::translate()
                                        .x(row_w - btns_w)
                                        .y((ROW_H - SMALL_BTN) / 2.0)
                                        .build(ui, |ui| {
                                            if small_btn(ui, row_id("q_up", i), SMALL_BTN, SMALL_BTN, "↑") {
                                                act = Some(Act::QueueUp(i));
                                            }
                                            nuon::translate().x(SMALL_BTN + 6.0).add_to_current(ui);
                                            if small_btn(ui, row_id("q_dn", i), SMALL_BTN, SMALL_BTN, "↓") {
                                                act = Some(Act::QueueDown(i));
                                            }
                                            nuon::translate().x(SMALL_BTN + 6.0).add_to_current(ui);
                                            if small_btn(ui, row_id("q_rm", i), SMALL_BTN, SMALL_BTN, "×") {
                                                act = Some(Act::QueueRemove(i));
                                            }
                                        });
                                });
                                nuon::translate().y(ROW_H + ROW_GAP).add_to_current(ui);
                            }
                        });
                });
            });

        // ── Barra inferior ──────────────────────────────────────────
        nuon::translate().x(0.0).y(win_h).build(ui, |ui| {
            nuon::translate().y(-10.0 - bottom_bar_h).add_to_current(ui);

            let gap = 10.0;
            let w = 80.0;
            let h = bottom_bar_h;

            nuon::translate().x(gap).build(ui, |ui| {
                if neo_btn_icon(ui, w, h, icons::left_arrow_icon()) {
                    self.state.go_back();
                }

                nuon::translate().x(w + gap).add_to_current(ui);
                if neo_btn()
                    .id(row_id("add_folder", 0))
                    .size(200.0, h)
                    .label("+ Pasta")
                    .build(ui)
                {
                    act = Some(Act::AddFolder);
                }

                nuon::translate().x(200.0 + gap).add_to_current(ui);
                if neo_btn()
                    .id(row_id("rescan", 0))
                    .size(160.0, h)
                    .label("Atualizar")
                    .build(ui)
                {
                    act = Some(Act::Rescan);
                }

                // Pastas ligadas (clicar remove)
                nuon::translate().x(160.0 + gap).add_to_current(ui);
                let mut x = 0.0;
                for (i, folder) in ctx.library.folders.iter().enumerate() {
                    let name = folder
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| folder.display().to_string());
                    let bw = 190.0;
                    if x + bw + w + gap * 3.0 > win_w - (w + gap) * 2.0 - 200.0 - 160.0 - w {
                        break;
                    }
                    nuon::translate().x(x).y(h / 2.0 - 15.0).build(ui, |ui| {
                        if small_btn(ui, row_id("folder", i), bw, 30.0, &format!("× {}", truncate(&name, 16))) {
                            act = Some(Act::RemoveFolder(i));
                        }
                    });
                    x += bw + 6.0;
                }
            });

            if !ctx.library.queue.is_empty() {
                nuon::translate().x(win_w - w - gap).build(ui, |ui| {
                    if neo_btn()
                        .id(row_id("play_playlist", 0))
                        .size(w, h)
                        .icon(icons::play_icon())
                        .tooltip("Tocar playlist")
                        .build(ui)
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
                if play_path(&self.state, ctx, path, None) {
                    // a cena muda; nada mais a fazer
                }
            }
            Act::PlayQueueFrom(i) => {
                if let Some(path) = ctx.library.queue.get(i).cloned() {
                    play_path(&self.state, ctx, path, Some(i));
                }
            }
            Act::Enqueue(path) => ctx.library.enqueue(path),
            Act::EnqueueAllVisible => {
                let paths: Vec<PathBuf> = visible
                    .iter()
                    .map(|&i| ctx.library.songs[i].path.clone())
                    .collect();
                ctx.library.enqueue_many(paths);
            }
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
