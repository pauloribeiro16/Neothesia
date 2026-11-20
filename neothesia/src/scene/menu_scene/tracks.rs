//! Página "Faixas": cartões de faixa com cor do instrumento, contagem de
//! notas e seletor Auto / Humano / Mudo.

use midi_file::MidiTrack;
use nuon::TextAlign;

use crate::{
    context::Context,
    icons,
    locals,
    song::{PlayerConfig, TrackConfig},
    theme,
    widgets,
};

use super::state;

pub const CARD_W: f32 = 344.0;
pub const CARD_H: f32 = 126.0;

impl super::MenuScene {
    pub fn tracks_page_ui(&mut self, ctx: &mut Context, ui: &mut nuon::Ui) {
        let win_w = ctx.window_state.logical_size.width;
        let win_h = ctx.window_state.logical_size.height;
        let bottom_bar_h = 60.0;
        let header_h = 56.0;

        // ── Cabeçalho ────────────────────────────────────────────────
        nuon::translate().x(theme::SPACE_XL).build(ui, |ui| {
            nuon::label()
                .y(theme::SPACE_S)
                .size(win_w - theme::SPACE_XL * 2.0, 44.0)
                .font_size(theme::FONT_XXL)
                .color(theme::text())
                .bold(true)
                .text_justify(nuon::TextAlign::Start)
                .text(locals::menu::tracks())
                .build(ui);
        });

        // ── Barra inferior ───────────────────────────────────────────
        nuon::translate().y(win_h - bottom_bar_h).build(ui, |ui| {
            let btn_h = 44.0;

            nuon::translate()
                .x(theme::SPACE_L)
                .y(nuon::center_y(bottom_bar_h, btn_h))
                .build(ui, |ui| {
                    if widgets::button(("tracks_back", 0), "")
                        .icon(icons::left_arrow_icon())
                        .size(btn_h, btn_h)
                        .build(ui)
                        .clicked
                    {
                        self.state.go_back();
                    }
                });

            nuon::translate()
                .x(win_w - theme::SPACE_L - btn_h)
                .y(nuon::center_y(bottom_bar_h, btn_h))
                .build(ui, |ui| {
                    if widgets::button(("tracks_play", 0), "")
                        .icon(icons::play_icon())
                        .size(btn_h, btn_h)
                        .primary()
                        .build(ui)
                        .clicked
                    {
                        state::play(&self.state, ctx);
                    }
                });
        });

        // ── Cartões ──────────────────────────────────────────────────
        let Some(song) = self.state.song.as_mut() else {
            return;
        };

        nuon::translate()
            .y(header_h + theme::SPACE_S)
            .build(ui, |ui| {
                nuon::scroll("tracks_scroll")
                    .scissor_size(win_w, (win_h - header_h - bottom_bar_h).max(0.0))
                    .build(ui, |ui| {
                        let gap = theme::SPACE_M;

                        let tracks = song
                            .file
                            .tracks
                            .iter()
                            .filter(|t| !t.notes.is_empty())
                            .enumerate();

                        let layout = CardsLayout::new(win_w, tracks.count());

                        nuon::translate().y(theme::SPACE_S).add_to_current(ui);

                        let mut tracks = song
                            .file
                            .tracks
                            .iter()
                            .filter(|t| !t.notes.is_empty())
                            .enumerate();

                        loop {
                            let mut end = false;
                            nuon::translate()
                                .x(nuon::center_x(win_w, layout.width))
                                .build(ui, |ui| {
                                    for _ in 0..layout.columns {
                                        let Some((id, track)) = tracks.next() else {
                                            end = true;
                                            break;
                                        };

                                        let config = &song.config.tracks[track.track_id];

                                        match self::track_card(
                                            ctx,
                                            ui,
                                            widgets::wid(("track_card", id)),
                                            track,
                                            config,
                                        ) {
                                            TrackCardEvent::PlayerConfig(player) => {
                                                song.config.tracks[track.track_id].player = player;
                                            }
                                            TrackCardEvent::SetVisible(visible) => {
                                                song.config.tracks[track.track_id].visible =
                                                    visible;
                                            }
                                            TrackCardEvent::Idle => {}
                                        }

                                        nuon::translate().x(CARD_W + gap).add_to_current(ui);
                                    }
                                });

                            nuon::translate().y(CARD_H + gap).add_to_current(ui);

                            if end {
                                break;
                            }
                        }
                    });
            });
    }
}

struct CardsLayout {
    columns: u8,
    width: f32,
}

impl CardsLayout {
    fn new(w: f32, tracks_count: usize) -> Self {
        const GAP: f32 = theme::SPACE_M;

        const LAYOUT_1: f32 = CARD_W;
        const LAYOUT_2: f32 = LAYOUT_1 + GAP + CARD_W;
        const LAYOUT_3: f32 = LAYOUT_2 + GAP + CARD_W;

        let columns = if w > LAYOUT_3 {
            3
        } else if w > LAYOUT_2 {
            2
        } else {
            1
        };

        let columns = columns.min(tracks_count).max(1) as u8;

        Self {
            columns,
            width: match columns {
                3 => LAYOUT_3,
                2 => LAYOUT_2,
                _ => LAYOUT_1,
            },
        }
    }
}

#[derive(Debug)]
enum TrackCardEvent {
    PlayerConfig(PlayerConfig),
    SetVisible(bool),
    Idle,
}

fn track_card(
    ctx: &Context,
    ui: &mut nuon::Ui,
    id: nuon::Id,
    track: &MidiTrack,
    config: &TrackConfig,
) -> TrackCardEvent {
    let pad = theme::SPACE_L;

    let track_color = if !config.visible {
        theme::text_faint()
    } else {
        let color_id = track.track_color_id % ctx.config.color_schema().len();
        let color = &ctx.config.color_schema()[color_id].base;
        nuon::Color::new_u8(color.0, color.1, color.2, 1.0)
    };

    let title = if track.has_drums && !track.has_other_than_drums {
        locals::tracks::percussion()
    } else {
        let instrument_id = track
            .programs
            .last()
            .map(|p| p.program as usize)
            .unwrap_or(0);
        midi_file::INSTRUMENT_NAMES[instrument_id].to_owned()
    };

    let subtitle = locals::tracks::notes(track.notes.len());

    theme::panel(ui, CARD_W, CARD_H);

    let inner_w = CARD_W - pad * 2.0;
    let mut res = TrackCardEvent::Idle;

    nuon::translate().pos(pad, pad).build(ui, |ui| {
        // Chip de cor: clicar alterna a faixa visível/muda
        if nuon::click_area(widgets::wid((id.as_raw(), "visible")))
            .size(30.0, 30.0)
            .build(ui)
            .is_clicked()
        {
            res = TrackCardEvent::SetVisible(!config.visible);
        }

        nuon::circle(15.0)
            .color(if config.visible {
                track_color
            } else {
                theme::surface_sunken()
            })
            .border_color(if config.visible {
                theme::border()
            } else {
                theme::border_strong()
            })
            .border_width(1.0)
            .build(ui);

        if !config.visible {
            nuon::label()
                .size(30.0, 30.0)
                .font_size(18.0)
                .color(theme::text_faint())
                .text("×")
                .build(ui);
        }

        // Nome do instrumento + contagem de notas
        nuon::translate().x(30.0 + theme::SPACE_M).build(ui, |ui| {
            let text_w = inner_w - 30.0 - theme::SPACE_M;
            nuon::label()
                .size(text_w, 20.0)
                .text_justify(TextAlign::Start)
                .font_size(theme::FONT_M)
                .color(theme::text())
                .text(theme::fit_text(&title, text_w, theme::FONT_M))
                .build(ui);
            nuon::label()
                .y(20.0)
                .size(text_w, 18.0)
                .text_justify(TextAlign::Start)
                .font_size(theme::FONT_S)
                .color(theme::text_muted())
                .text(subtitle)
                .build(ui);
        });

        // Seletor Auto / Humano / Mudo
        nuon::translate().y(56.0).build(ui, |ui| {
            let seg_h = 40.0;
            let track_w = inner_w;

            nuon::quad()
                .size(track_w, seg_h)
                .color(theme::surface_sunken())
                .border_color(theme::border())
                .border_width(1.0)
                .border_radius([theme::RADIUS_M; 4])
                .build(ui);

            let options = [
                (PlayerConfig::Auto, locals::tracks::auto()),
                (PlayerConfig::Human, locals::tracks::human()),
                (PlayerConfig::Mute, locals::tracks::mute()),
            ];
            let seg_w = (track_w - 6.0) / options.len() as f32;

            for (i, (mode, label)) in options.iter().enumerate() {
                let selected = *mode == config.player;
                let x = 3.0 + seg_w * i as f32;

                let (color, hover, preseed, font_color) = if selected {
                    (
                        theme::accent(),
                        theme::accent().mix(theme::accent_2(), 0.25),
                        theme::accent().mix(theme::accent_2(), 0.35),
                        theme::text(),
                    )
                } else {
                    (
                        theme::surface_sunken(),
                        theme::surface_hover(),
                        theme::surface_hover(),
                        theme::text_muted(),
                    )
                };

                if nuon::button()
                    .id(widgets::wid((id.as_raw(), format!("player-{i}"))))
                    .pos(x, 3.0)
                    .size(seg_w, seg_h - 6.0)
                    .color(color)
                    .hover_color(hover)
                    .preseed_color(preseed)
                    .font_color(font_color)
                    .border_radius([theme::RADIUS_S; 4])
                    .label_layout(
                        nuon::TextLayout::new(label.clone())
                            .size(theme::FONT_S)
                            .text_justify(TextAlign::Center)
                            .text_align(TextAlign::Center),
                    )
                    .build(ui)
                {
                    res = TrackCardEvent::PlayerConfig(*mode);
                }
            }
        });
    });

    res
}
