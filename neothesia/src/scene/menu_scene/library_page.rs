//! Página "Biblioteca": por autor (R2) — A ESTUDAR, autores, músicas e
//! favoritas, com pesquisa e painel da playlist.
//!
//! Teclado (em `library_page_window_event`): ↑/↓ cursor, Enter abre a música
//! (`open_song`), Tab junta à playlist, Esc limpa a pesquisa ou volta.

use std::{collections::HashMap, hash::Hash, path::PathBuf};

use midi_file::MidiFile;
use nuon::TextAlign;

use super::{MsgFn, UiState, on_async, state};
use crate::{
    NeothesiaEvent, context::Context, icons, library::Library, locals, recording::Take,
    song::Song, theme, utils::BoxFuture, widgets,
};

/// Largura do painel da playlist
const PANEL_W: f32 = 340.0;

/// Glifos bootstrap-icons v1.11.3 (a mover para `icons.rs`)
const STAR_ICON: &str = "\u{F588}";
const STAR_FILL_ICON: &str = "\u{F586}";
const ARROW_UP_ICON: &str = "\u{F148}";
const ARROW_DOWN_ICON: &str = "\u{F128}";

enum Act {
    OpenSong(PathBuf),
    Play(PathBuf),
    ToggleFavorite(PathBuf),
    Enqueue(PathBuf),
    Composer(String),
    QueueUp(usize),
    QueueDown(usize),
    QueueRemove(usize),
    ClearQueue,
    AddFolder,
}

/// Vista do seletor Autores / Músicas / Favoritas
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum View {
    Composers,
    Songs,
    Favorites,
}

impl View {
    const ALL: [Self; 3] = [Self::Composers, Self::Songs, Self::Favorites];

    fn label(self) -> String {
        match self {
            View::Composers => locals::library::composers(),
            View::Songs => locals::library::musics(),
            View::Favorites => locals::library::favorites_view(),
        }
    }
}

/// Estado local da página (sobrevive porque o id é usado todos os frames)
#[derive(Default)]
struct PageState {
    view: usize,
    /// Autor selecionado na vista Autores (chave `composer_key`)
    composer: Option<String>,
    playlist_open: bool,
}

fn page_state(ui: &mut nuon::Ui) -> &mut PageState {
    ui.data::<PageState>(widgets::wid(("library", "page_state")))
}

fn folder_name(path: &std::path::Path) -> String {
    path.parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// Uma gravação é "parcial" se cobre menos de metade do maior trecho
/// (mesma regra da `recordings_page`).
pub(super) fn is_partial(t: &Take, all: &[Take]) -> bool {
    let len = |t: &Take| {
        t.info
            .covered
            .map(|(a, b)| b.saturating_sub(a).as_secs_f32())
            .unwrap_or(0.0)
    };
    let longest = all.iter().map(len).fold(0.0, f32::max);
    longest > 0.0 && len(t) < longest * 0.5
}

/// Melhor precisão por música, com uma única leitura do disco por frame
fn best_accuracy_by_song(ctx: &Context) -> HashMap<PathBuf, f32> {
    let all = ctx.takes.list_all();
    let mut by_song: HashMap<PathBuf, Vec<&Take>> = HashMap::new();
    for t in &all {
        by_song.entry(t.song.clone()).or_default().push(t);
    }
    let mut map = HashMap::new();
    for (song, takes) in by_song {
        let owned: Vec<Take> = takes.into_iter().cloned().collect();
        let best = owned
            .iter()
            .filter(|t| !is_partial(t, &owned))
            .map(|t| t.accuracy)
            .fold(0.0_f32, f32::max);
        map.insert(song, best);
    }
    map
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
                self.state.open_song(ctx, path);
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
    /// Músicas que correspondem à pesquisa atual
    fn visible_songs<'a>(&self, ctx: &'a Context) -> Vec<&'a crate::library::LibSong> {
        let query = self.state.search.to_lowercase();
        ctx.library
            .songs
            .iter()
            .filter(|s| s.matches(&query))
            .collect()
    }

    pub fn library_page_ui(&mut self, ctx: &mut Context, ui: &mut nuon::Ui) {
        let win_h = ctx.window_state.logical_size.height;

        let (nav, nav_act) = self.side_nav(ctx, ui);
        let mut act: Option<Act> = None;

        // Estado local da página (com o clique no bloco da navegação)
        let ps = page_state(ui);
        if nav_act.open_playlist {
            ps.playlist_open = !ps.playlist_open;
        }
        let view = View::ALL[ps.view.min(2)];
        let selected_composer = ps.composer.clone();
        let playlist_open = ps.playlist_open;
        drop(ps);

        let best_acc = best_accuracy_by_song(ctx);

        // ── Cabeçalho ────────────────────────────────────────────────
        let search_w = 300.0_f32.max(160.0).min(nav.content_w - 400.0);
        let seg_w = 360.0_f32.min(nav.content_w - search_w - theme::SPACE_XL * 3.0);
        let body_w = nav.content_w - theme::SPACE_XL * 2.0;
        let x0 = nav.content_x + theme::SPACE_XL;

        nuon::translate().x(x0).y(theme::SPACE_S).build(ui, |ui| {
            widgets::page_title(ui, body_w, &locals::library::title());
            let n_authors = ctx.library.by_composer().len();
            nuon::label()
                .y(52.0)
                .size(body_w, 22.0)
                .font_size(theme::FONT_S)
                .color(theme::text_muted())
                .text_justify(TextAlign::Start)
                .text(locals::library::header_info(ctx.library.songs.len(), n_authors))
                .build(ui);
        });

        // Pesquisa + seletor de vista (segunda linha)
        nuon::translate()
            .x(nav.content_x + nav.content_w - theme::SPACE_XL - seg_w)
            .y(84.0)
            .build(ui, |ui| {
                if let Some(i) = self::segmented(ui, "lib_view", seg_w, &View::ALL, view) {
                    let ps = page_state(ui);
                    ps.view = i;
                    ps.composer = None;
                }
            });

        // Pesquisa, à esquerda do seletor
        nuon::translate()
            .x(nav.content_x + nav.content_w - theme::SPACE_XL - seg_w - theme::SPACE_M - search_w)
            .y(84.0)
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

        let content_y = 132.0;
        let content_h = (win_h - content_y - theme::SPACE_L).max(80.0);

        // ── Conteúdo conforme a vista ────────────────────────────────
        nuon::translate().x(x0).y(content_y).build(ui, |ui| {
            let query = self.state.search.to_lowercase();
            match view {
                View::Songs => {
                    let songs: Vec<crate::library::LibSong> = self
                        .visible_songs(ctx)
                        .into_iter()
                        .cloned()
                        .collect();
                    let cursor = self.state.lib_cursor.min(songs.len().saturating_sub(1));
                    songs_list(
                        ctx,
                        ui,
                        body_w,
                        content_h,
                        &best_acc,
                        &songs,
                        Some(cursor),
                        &mut act,
                    );
                }
                View::Favorites => {
                    let songs: Vec<crate::library::LibSong> = ctx
                        .library
                        .songs
                        .iter()
                        .filter(|s| s.matches(&query) && ctx.library.is_favorite(&s.path))
                        .cloned()
                        .collect();
                    songs_list(ctx, ui, body_w, content_h, &best_acc, &songs, None, &mut act);
                }
                View::Composers => {
                    if let Some(key) = &selected_composer {
                        composer_songs(self, ctx, ui, body_w, content_h, key, &best_acc, &mut act);
                    } else {
                        composers_view(ctx, ui, body_w, content_h, &query, &best_acc, &mut act);
                    }
                }
            }
        });

        // ── Painel da playlist (abre a partir do bloco da navegação) ──
        if playlist_open {
            playlist_panel(ctx, ui, nav.w, win_h, &mut act);
        }

        self.apply_nav_actions(ctx, nav_act);

        // ── Aplicar ação ─────────────────────────────────────────────
        let Some(act) = act else { return };
        match act {
            Act::OpenSong(path) => self.state.open_song(ctx, path),
            Act::Play(path) => {
                play_path(&self.state, ctx, path, None);
            }
            Act::ToggleFavorite(path) => ctx.library.toggle_favorite(&path),
            Act::Enqueue(path) => ctx.library.enqueue(path),
            Act::Composer(key) => {
                let ps = page_state(ui);
                ps.composer = Some(key);
            }
            Act::QueueUp(i) => ctx.library.move_in_queue(i, true),
            Act::QueueDown(i) => ctx.library.move_in_queue(i, false),
            Act::QueueRemove(i) => ctx.library.remove_from_queue(i),
            Act::ClearQueue => ctx.library.clear_queue(),
            Act::AddFolder => self.futures.push(pick_folder_fut()),
        }
    }
}

/// Seletor Autores / Músicas / Favoritas
pub(super) fn segmented(
    ui: &mut nuon::Ui,
    id: &str,
    w: f32,
    options: &[View],
    selected: View,
) -> Option<usize> {
    let labels: Vec<String> = options.iter().map(|v| v.label()).collect();
    let sel = View::ALL.iter().position(|v| *v == selected).unwrap_or(0);
    segmented_labels(ui, id, w, &labels, sel)
}

/// Seletor segmentado genérico (também usado na página da música)
pub(super) fn segmented_labels(
    ui: &mut nuon::Ui,
    id: &str,
    w: f32,
    labels: &[String],
    selected: usize,
) -> Option<usize> {
    let h = 36.0;
    let pad = 3.0;

    let layouts: Vec<_> = labels
        .iter()
        .map(|l| {
            nuon::TextLayout::new(l.clone())
                .size(theme::FONT_S)
                .text_justify(TextAlign::Center)
                .text_align(TextAlign::Center)
        })
        .collect();
    let seg_w = (w - pad * 2.0) / layouts.len() as f32;

    nuon::quad()
        .size(w, h)
        .color(theme::surface_sunken())
        .border_color(theme::border())
        .border_width(1.0)
        .border_radius([theme::RADIUS_M; 4])
        .build(ui);

    let mut clicked = None;
    for (i, layout) in layouts.into_iter().enumerate() {
        let is_sel = i == selected;
        let (color, hover, preseed, font_color) = if is_sel {
            (
                theme::accent(),
                theme::accent().mix(theme::accent_2(), 0.25),
                theme::accent().mix(theme::accent_2(), 0.35),
                theme::on_accent(),
            )
        } else {
            (
                nuon::Color::new_u8(0, 0, 0, 0.0),
                theme::surface_hover(),
                theme::surface_hover(),
                theme::text_muted(),
            )
        };

        if nuon::button()
            .id(format!("{id}-{i}"))
            .pos(pad + seg_w * i as f32, pad)
            .size(seg_w, h - pad * 2.0)
            .color(color)
            .hover_color(hover)
            .preseed_color(preseed)
            .font_color(font_color)
            .border_radius([theme::RADIUS_S; 4])
            .label_layout(layout)
            .build(ui)
        {
            clicked = Some(i);
        }
    }
    clicked
}

/// Botão pequeno de ícone (devolve o builder para o chamador encadear
/// `.tooltip(...).build(ui)`)
fn icon_btn(ui: &mut nuon::Ui, id: impl Hash, icon: &str, size: f32) -> widgets::Button {
    let _ = ui;
    widgets::button(id, "").icon(icon).size(size, size).ghost()
}

/// Linhas de música (vista Músicas/Favoritas e músicas de um autor)
fn songs_list(
    ctx: &mut Context,
    ui: &mut nuon::Ui,
    body_w: f32,
    content_h: f32,
    best_acc: &HashMap<PathBuf, f32>,
    songs: &[crate::library::LibSong],
    cursor: Option<usize>,
    act: &mut Option<Act>,
) {
    if ctx.library.folders.is_empty() {
        widgets::hint(ui, body_w, 40.0, &locals::library::empty_hint());
        nuon::translate().y(theme::SPACE_M).add_to_current(ui);
        if widgets::button(("lib_add", 0), locals::library::add_folder_action())
            .primary()
            .build(ui)
            .clicked
        {
            *act = Some(Act::AddFolder);
        }
        return;
    }
    if songs.is_empty() {
        let text = if ctx.library.songs.is_empty() {
            locals::library::empty_hint()
        } else {
            locals::library::no_results()
        };
        widgets::hint(ui, body_w, 40.0, &text);
        return;
    }

    let trailing = 36.0 * 3.0 + theme::SPACE_S * 2.0;
    let row_w = body_w - 14.0;
    let playing_path = ctx
        .library
        .queue_pos
        .and_then(|i| ctx.library.queue.get(i))
        .cloned();

    nuon::scroll("library_scroll")
        .scissor_size(body_w, content_h)
        .build(ui, |ui| {
            for (pos, song) in songs.iter().enumerate() {
                let (title, composer) = Library::pretty_title(&song.path);
                let sub = composer.unwrap_or_else(|| folder_name(&song.path));
                let is_playing = playing_path.as_ref() == Some(&song.path);
                let is_fav = ctx.library.is_favorite(&song.path);

                let mut row = widgets::list_row(("lib_row", pos), row_w, &title)
                    .subtitle(&sub)
                    .selected(cursor == Some(pos))
                    .active(is_playing)
                    .trailing(trailing);
                if let Some(acc) = best_acc.get(&song.path)
                    && *acc > 0.0
                {
                    row = row.leading(format!("{:.0}%", acc * 100.0));
                }
                row.build(ui);

                // Trailing: [▶] [★] [+]
                nuon::translate().x(row_w - trailing).build(ui, |ui| {
                    if icon_btn(ui, ("lib_play", pos), icons::play_icon(), 36.0)
                        .tooltip(locals::library::play_now())
                        .build(ui)
                        .clicked
                    {
                        *act = Some(Act::Play(song.path.clone()));
                    }
                    nuon::translate().x(36.0 + theme::SPACE_S).add_to_current(ui);

                    let star = if is_fav { STAR_FILL_ICON } else { STAR_ICON };
                    if icon_btn(ui, ("lib_fav", pos), star, 36.0)
                        .tooltip(if is_fav {
                            locals::library::favorite_remove()
                        } else {
                            locals::library::favorite_add()
                        })
                        .build(ui)
                        .clicked
                    {
                        *act = Some(Act::ToggleFavorite(song.path.clone()));
                    }
                    nuon::translate().x(36.0 + theme::SPACE_S).add_to_current(ui);

                    if icon_btn(ui, ("lib_add", pos), icons::plus_icon(), 36.0)
                        .tooltip(locals::library::enqueue())
                        .build(ui)
                        .clicked
                    {
                        *act = Some(Act::Enqueue(song.path.clone()));
                    }
                });

                nuon::translate()
                    .y(theme::ROW_H + theme::SPACE_XS)
                    .add_to_current(ui);
            }
        });
}

/// Vista Autores: A ESTUDAR (favoritas em cartões) + grelha de monogramas
fn composers_view(
    ctx: &mut Context,
    ui: &mut nuon::Ui,
    body_w: f32,
    content_h: f32,
    query: &str,
    best_acc: &HashMap<PathBuf, f32>,
    act: &mut Option<Act>,
) {
    let groups: Vec<crate::library::ComposerGroup> = ctx
        .library
        .by_composer()
        .into_iter()
        .filter(|g| {
            query.is_empty()
                || g.name.to_lowercase().contains(query)
                || g.songs
                    .iter()
                    .any(|&i| ctx.library.songs[i].matches(query))
        })
        .collect();

    // A ESTUDAR = favoritas, em cartões grandes
    let favs: Vec<&crate::library::LibSong> = ctx
        .library
        .songs
        .iter()
        .filter(|s| s.matches(query) && ctx.library.is_favorite(&s.path))
        .collect();

    nuon::scroll("library_scroll")
        .scissor_size(body_w, content_h)
        .build(ui, |ui| {
            let row_w = body_w - 14.0;

            if !favs.is_empty() {
                widgets::eyebrow(ui, row_w, &locals::library::to_study());
                nuon::translate().y(24.0).add_to_current(ui);

                let cols = 3;
                let gap = theme::SPACE_S;
                let card_w = ((row_w - gap * (cols - 1) as f32) / cols as f32).min(320.0);
                let card_h = 96.0;
                for (i, song) in favs.iter().enumerate() {
                    let col = i % cols;
                    let row = i / cols;
                    nuon::translate()
                        .x(col as f32 * (card_w + gap))
                        .y(row as f32 * (card_h + gap))
                        .build(ui, |ui| {
                            study_card(ui, ("lib_study", i), card_w, card_h, song, best_acc, act);
                        });
                }
                let rows = favs.len().div_ceil(cols);
                nuon::translate()
                    .y(rows as f32 * (card_h + gap))
                    .add_to_current(ui);
                nuon::translate().y(theme::SPACE_XL).add_to_current(ui);
            }

            if groups.is_empty() {
                widgets::hint(ui, row_w, 40.0, &locals::library::no_results());
                return;
            }

            widgets::eyebrow(ui, row_w, &locals::library::composers());
            nuon::translate().y(24.0).add_to_current(ui);

            let cols = 4;
            let gap = theme::SPACE_M;
            let card_w = (row_w - gap * (cols - 1) as f32) / cols as f32;
            let card_h = 76.0;
            for (i, g) in groups.iter().enumerate() {
                let col = i % cols;
                let row = i / cols;
                nuon::translate()
                    .x(col as f32 * (card_w + gap))
                    .y(row as f32 * (card_h + gap))
                    .build(ui, |ui| {
                        composer_card(ui, ("lib_composer", i), card_w, card_h, g, act);
                    });
            }
        });
}

/// Cartão de favorita: título em serifa, autor em dourado, precisão e ▶
fn study_card(
    ui: &mut nuon::Ui,
    id: impl Hash,
    w: f32,
    h: f32,
    song: &crate::library::LibSong,
    best_acc: &HashMap<PathBuf, f32>,
    act: &mut Option<Act>,
) {
    let (title, composer) = Library::pretty_title(&song.path);
    let composer = composer.unwrap_or_default();
    let acc = best_acc.get(&song.path).copied().filter(|a| *a > 0.0);

    let r = widgets::card(ui, id, w, h, |ui, t| {
        let title_w = w - 120.0;
        nuon::label()
            .x(theme::SPACE_L)
            .y(theme::SPACE_M)
            .size(title_w, 30.0)
            .font_size(theme::FONT_L)
            .font_family(theme::FONT_DISPLAY)
            .color(theme::text())
            .text_justify(TextAlign::Start)
            .text(theme::fit_text(&title, title_w, theme::FONT_L))
            .build(ui);
        nuon::label()
            .x(theme::SPACE_L)
            .y(42.0)
            .size(title_w, 22.0)
            .font_size(theme::FONT_S)
            .color(theme::accent().mix(theme::text(), t * 0.3))
            .text_justify(TextAlign::Start)
            .text(theme::fit_text(&composer, title_w, theme::FONT_S))
            .build(ui);

        // Precisão (anel aproximado: círculo com borda dourada + valor)
        if let Some(acc) = acc {
            let cx = w - 96.0;
            nuon::circle(20.0)
                .pos(cx, h / 2.0 - 20.0)
                .color(nuon::Color::new_u8(0, 0, 0, 0.0))
                .border_color(theme::accent().mix(theme::surface_sunken(), 0.75))
                .border_width(2.0)
                .build(ui);
            nuon::label()
                .pos(cx - 20.0, h / 2.0 - 11.0)
                .size(40.0, 22.0)
                .font_size(theme::FONT_S - 1.0)
                .bold(true)
                .color(theme::accent())
                .text(format!("{:.0}%", acc * 100.0))
                .build(ui);
        }

        // ▶ redondo dourado
        nuon::translate()
            .x(w - 36.0 - theme::SPACE_L)
            .y(h / 2.0 - 18.0)
            .build(ui, |ui| {
                if widgets::button(("study_play", title.clone()), "")
                    .icon(icons::play_icon())
                    .primary()
                    .size(36.0, 36.0)
                    .build(ui)
                    .clicked
                {
                    *act = Some(Act::Play(song.path.clone()));
                }
            });
    });
    if r.clicked {
        *act = Some(Act::OpenSong(song.path.clone()));
    }
}

/// Cartão de autor: monograma redondo + nome + N músicas
fn composer_card(
    ui: &mut nuon::Ui,
    id: impl Hash,
    w: f32,
    h: f32,
    group: &crate::library::ComposerGroup,
    act: &mut Option<Act>,
) {
    let initial = group
        .name
        .split_whitespace()
        .last()
        .and_then(|w| w.chars().next())
        .unwrap_or('?')
        .to_uppercase()
        .to_string();
    let count = locals::library::n_songs(group.songs.len());
    let is_others = group.key.is_empty();
    let name = if is_others {
        locals::library::others()
    } else {
        group.name.clone()
    };

    let r = widgets::card(ui, id, w, h, |ui, _t| {
        nuon::circle(24.0)
            .pos(theme::SPACE_M + 2.0, h / 2.0 - 24.0)
            .color(theme::surface_sunken())
            .border_color(theme::border_strong())
            .border_width(1.0)
            .build(ui);
        nuon::label()
            .pos(theme::SPACE_M + 2.0, h / 2.0 - 15.0)
            .size(48.0, 30.0)
            .font_size(theme::FONT_L)
            .font_family(theme::FONT_DISPLAY)
            .color(theme::text_muted())
            .text(initial)
            .build(ui);

        let text_x = theme::SPACE_M * 2.0 + 48.0 + theme::SPACE_M;
        let text_w = w - text_x - theme::SPACE_M;
        nuon::label()
            .pos(text_x, h / 2.0 - 22.0)
            .size(text_w, 26.0)
            .font_size(theme::FONT_M)
            .font_family(theme::FONT_DISPLAY)
            .color(theme::text())
            .text_justify(TextAlign::Start)
            .text(theme::fit_text(&name, text_w, theme::FONT_M))
            .build(ui);
        nuon::label()
            .pos(text_x, h / 2.0 + 2.0)
            .size(text_w, 20.0)
            .font_size(theme::FONT_S)
            .color(theme::text_muted())
            .text_justify(TextAlign::Start)
            .text(count)
            .build(ui);
    });
    if r.clicked && !is_others {
        *act = Some(Act::Composer(group.key.clone()));
    }
}

/// Músicas de um autor selecionado
fn composer_songs(
    this: &mut super::MenuScene,
    ctx: &mut Context,
    ui: &mut nuon::Ui,
    body_w: f32,
    content_h: f32,
    key: &str,
    best_acc: &HashMap<PathBuf, f32>,
    act: &mut Option<Act>,
) {
    let groups = ctx.library.by_composer();
    let Some(group) = groups.iter().find(|g| g.key == key) else {
        return;
    };
    let name = if group.key.is_empty() {
        locals::library::others()
    } else {
        group.name.clone()
    };

    // Migalhas: Autores › Autor
    if widgets::button(("lib_crumb", 0), locals::library::composers())
        .ghost()
        .font_size(theme::FONT_S)
        .size(120.0, 28.0)
        .build(ui)
        .clicked
    {
        let ps = page_state(ui);
        ps.composer = None;
    }
    nuon::label()
        .x(124.0)
        .size(20.0, 28.0)
        .font_size(theme::FONT_S)
        .color(theme::text_faint())
        .text("›")
        .build(ui);
    nuon::label()
        .x(146.0)
        .size(body_w - 146.0, 28.0)
        .font_size(theme::FONT_S)
        .color(theme::text())
        .text_justify(TextAlign::Start)
        .text(name)
        .build(ui);
    nuon::translate().y(36.0).add_to_current(ui);

    let songs: Vec<crate::library::LibSong> = group
        .songs
        .iter()
        .filter_map(|&i| {
            let s = &ctx.library.songs[i];
            s.matches(&this.state.search.to_lowercase())
                .then(|| s.clone())
        })
        .collect();
    songs_list(ctx, ui, body_w, content_h - 36.0, best_acc, &songs, None, act);
}

/// Painel da playlist, aberto a partir do bloco da navegação
fn playlist_panel(
    ctx: &mut Context,
    ui: &mut nuon::Ui,
    nav_w: f32,
    win_h: f32,
    act: &mut Option<Act>,
) {
    let panel_h = (win_h * 0.7).max(300.0);
    nuon::layer().overlay(true).build(ui, |ui| {
        nuon::translate()
            .x(nav_w + theme::SPACE_S)
            .y(win_h - panel_h - theme::SPACE_L)
            .build(ui, |ui| {
                theme::panel(ui, PANEL_W, panel_h);

                nuon::translate()
                    .x(theme::SPACE_L)
                    .y(theme::SPACE_L)
                    .build(ui, |ui| {
                        let inner_w = PANEL_W - theme::SPACE_L * 2.0;

                        widgets::section_title(
                            ui,
                            inner_w,
                            &locals::library::playlist(),
                            Some(ctx.library.queue.len()),
                        );
                        nuon::translate().x(inner_w - 84.0).build(ui, |ui| {
                            if widgets::button(("pl_clear", 0), locals::library::clear())
                                .size(84.0, 28.0)
                                .ghost()
                                .font_size(theme::FONT_S)
                                .build(ui)
                                .clicked
                            {
                                *act = Some(Act::ClearQueue);
                            }
                        });

                        let list_h = panel_h - theme::SPACE_L * 3.0 - 28.0 - theme::BUTTON_H;
                        nuon::translate().y(28.0 + theme::SPACE_S).add_to_current(ui);

                        if ctx.library.queue.is_empty() {
                            widgets::hint(ui, inner_w, 60.0, &locals::library::queue_empty_hint());
                        } else {
                            nuon::scroll("playlist_scroll")
                                .scissor_size(inner_w, list_h)
                                .build(ui, |ui| {
                                    let row_w = inner_w - 14.0;
                                    let trailing = 36.0 * 3.0 + theme::SPACE_S * 2.0;
                                    for (i, path) in ctx.library.queue.iter().enumerate() {
                                        let (title, _) = Library::pretty_title(path);
                                        widgets::list_row(("pl_row", i), row_w, &title)
                                            .leading(format!("{}", i + 1))
                                            .active(ctx.library.queue_pos == Some(i))
                                            .trailing(trailing)
                                            .build(ui);

                                        nuon::translate().x(row_w - trailing).build(ui, |ui| {
                                            if icon_btn(ui, ("pl_up", i), ARROW_UP_ICON, 36.0)
                                                .tooltip(locals::library::queue_up())
                                                .build(ui)
                                                .clicked
                                            {
                                                *act = Some(Act::QueueUp(i));
                                            }
                                            nuon::translate()
                                                .x(36.0 + theme::SPACE_S)
                                                .add_to_current(ui);
                                            if icon_btn(ui, ("pl_dn", i), ARROW_DOWN_ICON, 36.0)
                                                .tooltip(locals::library::queue_down())
                                                .build(ui)
                                                .clicked
                                            {
                                                *act = Some(Act::QueueDown(i));
                                            }
                                            nuon::translate()
                                                .x(36.0 + theme::SPACE_S)
                                                .add_to_current(ui);
                                            if icon_btn(ui, ("pl_rm", i), icons::trash_icon(), 36.0)
                                                .tooltip(locals::library::remove())
                                                .build(ui)
                                                .clicked
                                            {
                                                *act = Some(Act::QueueRemove(i));
                                            }
                                        });

                                        nuon::translate()
                                            .y(theme::ROW_H + theme::SPACE_XS)
                                            .add_to_current(ui);
                                    }
                                });
                        }

                        if !ctx.library.queue.is_empty() {
                            nuon::translate().y(list_h + theme::SPACE_S).add_to_current(ui);
                            if widgets::button(("pl_play", 0), locals::library::play_playlist())
                                .icon(icons::play_icon())
                                .primary()
                                .size(inner_w, theme::BUTTON_H)
                                .build(ui)
                                .clicked
                            {
                                *act = Some(Act::Play(ctx.library.queue[0].clone()));
                            }
                        }
                    });
            });
    });
}
