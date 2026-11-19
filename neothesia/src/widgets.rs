//! Componentes de UI reutilizáveis, construídos sobre `nuon` + `theme`.
//!
//! Regra: os ecrãs (menu, biblioteca, definições…) **não** desenham quads/labels
//! soltos para coisas comuns; usam estes componentes. Ver `ARQUITETURA.md`.
//!
//! Todos seguem o padrão builder do `nuon`: `widgets::x(id, …).opção().build(ui)`.
//! O desenho é feito na posição atual (usar `nuon::translate()` para posicionar).

#![allow(dead_code)]

use std::hash::Hash;

use nuon::{Color, TextAlign};

use crate::theme;

/// Cria um `nuon::Id` a partir de qualquer coisa que implemente `Hash`
/// (ex.: `wid(("lib_row", i))`).
pub fn wid(v: impl Hash) -> nuon::Id {
    nuon::Id::hash(v)
}

/// Resposta de um componente clicável.
#[derive(Debug, Clone, Copy, Default)]
pub struct Response {
    pub clicked: bool,
    pub hovered: bool,
    pub pressed: bool,
    /// 0..1 animado (hover)
    pub hover_t: f32,
}

fn interact(ui: &mut nuon::Ui, id: nuon::Id, w: f32, h: f32) -> Response {
    let ev = nuon::click_area(id).size(w, h).build(ui);
    let hovered = ev.is_hovered();
    let pressed = ev.is_pressed();
    let hover_t = theme::anim(ui, ("hover", id.as_raw()), (hovered || pressed) as u8 as f32, 0.25);
    Response {
        clicked: ev.is_clicked(),
        hovered,
        pressed,
        hover_t,
    }
}

// ── Botão ───────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    /// Fundo com acento (ação principal do ecrã; no máx. 1 por ecrã)
    Primary,
    /// Superfície normal
    Secondary,
    /// Só texto/ícone, fundo aparece no hover
    Ghost,
    /// Ação destrutiva
    Danger,
}

pub struct Button {
    id: nuon::Id,
    w: f32,
    h: f32,
    label: String,
    icon: Option<String>,
    kind: ButtonKind,
    font_size: f32,
    tooltip: Option<String>,
}

/// Botão de texto e/ou ícone. `icon` usa os glifos de `crate::icons`.
pub fn button(id: impl Hash, label: impl Into<String>) -> Button {
    Button {
        id: wid(id),
        w: 160.0,
        h: theme::BUTTON_H,
        label: label.into(),
        icon: None,
        kind: ButtonKind::Secondary,
        font_size: theme::FONT_M,
        tooltip: None,
    }
}

impl Button {
    pub fn size(mut self, w: f32, h: f32) -> Self {
        self.w = w;
        self.h = h;
        self
    }
    pub fn icon(mut self, icon: &str) -> Self {
        self.icon = Some(icon.to_owned());
        self
    }
    pub fn kind(mut self, kind: ButtonKind) -> Self {
        self.kind = kind;
        self
    }
    pub fn primary(self) -> Self {
        self.kind(ButtonKind::Primary)
    }
    pub fn ghost(self) -> Self {
        self.kind(ButtonKind::Ghost)
    }
    pub fn danger(self) -> Self {
        self.kind(ButtonKind::Danger)
    }
    pub fn font_size(mut self, s: f32) -> Self {
        self.font_size = s;
        self
    }
    pub fn tooltip(mut self, t: impl Into<String>) -> Self {
        self.tooltip = Some(t.into());
        self
    }

    pub fn build(self, ui: &mut nuon::Ui) -> Response {
        let (w, h) = (self.w, self.h);
        let r = interact(ui, self.id, w, h);
        let t = r.hover_t;

        let (bg, fg, border) = match self.kind {
            ButtonKind::Primary => (
                theme::accent().mix(theme::accent_2(), t),
                theme::text(),
                Color::new_u8(255, 255, 255, 0.0),
            ),
            ButtonKind::Secondary => (
                theme::surface().mix(theme::surface_hover(), t),
                theme::text(),
                theme::border().mix(theme::border_strong(), t),
            ),
            ButtonKind::Ghost => (
                Color::new_u8(255, 255, 255, 0.0).mix(theme::surface_hover(), t),
                theme::text_muted().mix(theme::text(), t),
                Color::new_u8(255, 255, 255, 0.0),
            ),
            ButtonKind::Danger => (
                theme::surface().mix(theme::danger(), t * 0.85),
                theme::danger().mix(theme::text(), t),
                theme::danger().mix(theme::border(), 0.6),
            ),
        };

        let lift = if r.pressed { 0.0 } else { -1.5 * t };
        nuon::translate().y(lift).build(ui, |ui| {
            nuon::quad()
                .size(w, h)
                .color(bg)
                .border_color(border)
                .border_width(1.0)
                .border_radius([theme::RADIUS_M; 4])
                .build(ui);

            match (&self.icon, self.label.is_empty()) {
                (Some(icon), true) => {
                    nuon::label()
                        .size(w, h)
                        .font_size(self.font_size + 4.0)
                        .color(fg)
                        .icon(icon)
                        .build(ui);
                }
                (Some(icon), false) => {
                    let icon_w = h * 0.8;
                    nuon::label()
                        .x(theme::SPACE_S)
                        .size(icon_w, h)
                        .font_size(self.font_size + 2.0)
                        .color(fg)
                        .icon(icon)
                        .build(ui);
                    nuon::label()
                        .x(theme::SPACE_S + icon_w)
                        .size(w - icon_w - theme::SPACE_S * 2.0, h)
                        .font_size(self.font_size)
                        .color(fg)
                        .text_justify(TextAlign::Start)
                        .text(theme::fit_text(&self.label, w - icon_w - 16.0, self.font_size))
                        .build(ui);
                }
                (None, _) => {
                    nuon::label()
                        .size(w, h)
                        .font_size(self.font_size)
                        .color(fg)
                        .text(theme::fit_text(&self.label, w - 16.0, self.font_size))
                        .build(ui);
                }
            }
        });

        if let Some(tip) = &self.tooltip
            && r.hovered
        {
            tooltip(ui, tip, w);
        }

        r
    }
}

/// Botão quadrado só com ícone (atalho para `button(..).icon(..)`).
pub fn icon_button(ui: &mut nuon::Ui, id: impl Hash, icon: &str, size: f32) -> Response {
    button(id, "").icon(icon).size(size, size).ghost().build(ui)
}

fn tooltip(ui: &mut nuon::Ui, text: &str, anchor_w: f32) {
    let w = (text.chars().count() as f32 * theme::FONT_S * 0.6 + 16.0).max(40.0);
    let h = 24.0;
    nuon::layer().overlay(true).build(ui, |ui| {
        nuon::translate()
            .x((anchor_w - w) / 2.0)
            .y(-h - 6.0)
            .build(ui, |ui| {
                nuon::quad()
                    .size(w, h)
                    .color(Color::new_u8(8, 8, 12, 0.95))
                    .border_color(theme::border_strong())
                    .border_width(1.0)
                    .border_radius([theme::RADIUS_S; 4])
                    .build(ui);
                nuon::label()
                    .size(w, h)
                    .font_size(theme::FONT_S)
                    .color(theme::text())
                    .text(text)
                    .build(ui);
            });
    });
}

// ── Linha de lista ──────────────────────────────────────────────────

pub struct ListRow<'a> {
    id: nuon::Id,
    w: f32,
    h: f32,
    title: &'a str,
    subtitle: Option<&'a str>,
    leading: Option<String>,
    selected: bool,
    active: bool,
    trailing_w: f32,
}

/// Linha de lista (músicas, playlist, pastas…).
///
/// - `selected`: foco do teclado (borda azul)
/// - `active`: item "a tocar"/escolhido (barra roxa à esquerda)
/// - `trailing(w)`: reserva `w` px à direita para botões que o chamador desenha
///   depois (com `nuon::translate().x(row_w - w)`). O clique nessa zona não conta
///   como clique na linha.
pub fn list_row<'a>(id: impl Hash, w: f32, title: &'a str) -> ListRow<'a> {
    ListRow {
        id: wid(id),
        w,
        h: theme::ROW_H,
        title,
        subtitle: None,
        leading: None,
        selected: false,
        active: false,
        trailing_w: 0.0,
    }
}

impl<'a> ListRow<'a> {
    pub fn height(mut self, h: f32) -> Self {
        self.h = h;
        self
    }
    pub fn subtitle(mut self, s: &'a str) -> Self {
        self.subtitle = Some(s);
        self
    }
    /// Texto curto à esquerda (ex.: número "3" ou "▶")
    pub fn leading(mut self, s: impl Into<String>) -> Self {
        self.leading = Some(s.into());
        self
    }
    pub fn selected(mut self, v: bool) -> Self {
        self.selected = v;
        self
    }
    pub fn active(mut self, v: bool) -> Self {
        self.active = v;
        self
    }
    pub fn trailing(mut self, w: f32) -> Self {
        self.trailing_w = w;
        self
    }

    pub fn build(self, ui: &mut nuon::Ui) -> Response {
        let (w, h) = (self.w, self.h);
        let click_w = (w - self.trailing_w).max(0.0);
        let r = interact(ui, self.id, click_w, h);
        let t = r.hover_t;

        let base = if self.active {
            theme::surface_active()
        } else {
            theme::surface_sunken()
        };
        let border = if self.selected {
            theme::accent_2()
        } else {
            theme::border().mix(theme::border_strong(), t)
        };

        nuon::quad()
            .size(w, h)
            .color(base.mix(theme::surface_hover(), t * 0.8))
            .border_color(border)
            .border_width(if self.selected { 1.5 } else { 1.0 })
            .border_radius([theme::RADIUS_M; 4])
            .build(ui);

        if self.active {
            nuon::quad()
                .pos(0.0, 8.0)
                .size(3.0, h - 16.0)
                .color(theme::accent())
                .border_radius([2.0; 4])
                .build(ui);
        }

        let mut x = theme::SPACE_L;
        if let Some(lead) = &self.leading {
            let lw = 28.0;
            nuon::label()
                .x(x)
                .size(lw, h)
                .font_size(theme::FONT_S)
                .color(if self.active { theme::accent() } else { theme::text_faint() })
                .text(lead.clone())
                .build(ui);
            x += lw + theme::SPACE_S;
        }

        let text_w = (w - x - self.trailing_w - theme::SPACE_S).max(10.0);
        match self.subtitle {
            Some(sub) => {
                nuon::label()
                    .pos(x, h * 0.5 - 20.0)
                    .size(text_w, 22.0)
                    .font_size(theme::FONT_M)
                    .color(theme::text())
                    .text_justify(TextAlign::Start)
                    .text(theme::fit_text(self.title, text_w, theme::FONT_M))
                    .build(ui);
                nuon::label()
                    .pos(x, h * 0.5 + 1.0)
                    .size(text_w, 18.0)
                    .font_size(theme::FONT_S)
                    .color(theme::text_muted())
                    .text_justify(TextAlign::Start)
                    .text(theme::fit_text(sub, text_w, theme::FONT_S))
                    .build(ui);
            }
            None => {
                nuon::label()
                    .x(x)
                    .size(text_w, h)
                    .font_size(theme::FONT_M)
                    .color(theme::text())
                    .text_justify(TextAlign::Start)
                    .text(theme::fit_text(self.title, text_w, theme::FONT_M))
                    .build(ui);
            }
        }

        r
    }
}

// ── Campo de pesquisa (só desenho; o texto vem do teclado no window_event) ──

/// Desenha uma caixa de pesquisa. O texto é gerido pelo ecrã (ver
/// `ARQUITETURA.md` › Entrada de texto). Devolve `true` se clicaram no ✕.
pub fn search_box(ui: &mut nuon::Ui, id: impl Hash, w: f32, text: &str, placeholder: &str) -> bool {
    let id = wid(id);
    let h = theme::BUTTON_H;
    nuon::quad()
        .size(w, h)
        .color(theme::surface_sunken())
        .border_color(if text.is_empty() { theme::border() } else { theme::accent_2() })
        .border_width(1.0)
        .border_radius([theme::RADIUS_M; 4])
        .build(ui);

    nuon::label()
        .x(theme::SPACE_M)
        .size(24.0, h)
        .font_size(theme::FONT_M)
        .color(theme::text_faint())
        .text("⌕")
        .build(ui);

    let tx = theme::SPACE_M + 28.0;
    let tw = w - tx - h;
    let (shown, color) = if text.is_empty() {
        (placeholder.to_owned(), theme::text_faint())
    } else {
        (format!("{text}▏"), theme::text())
    };
    nuon::label()
        .x(tx)
        .size(tw, h)
        .font_size(theme::FONT_M)
        .color(color)
        .text_justify(TextAlign::Start)
        .text(theme::fit_text(&shown, tw, theme::FONT_M))
        .build(ui);

    if text.is_empty() {
        return false;
    }
    let mut clear = false;
    nuon::translate().x(w - h).build(ui, |ui| {
        clear = button((id.as_raw(), "clear"), "×")
            .size(h, h)
            .ghost()
            .font_size(theme::FONT_L)
            .build(ui)
            .clicked;
    });
    clear
}

// ── Texto ───────────────────────────────────────────────────────────

/// Título de secção (ex.: "Playlist", "Recentes"), com `count` opcional.
pub fn section_title(ui: &mut nuon::Ui, w: f32, text: &str, count: Option<usize>) {
    nuon::label()
        .size(w, 28.0)
        .font_size(theme::FONT_L)
        .color(theme::text())
        .bold(true)
        .text_justify(TextAlign::Start)
        .text(text)
        .build(ui);
    if let Some(n) = count {
        nuon::label()
            .size(w, 28.0)
            .font_size(theme::FONT_S)
            .color(theme::text_muted())
            .text_justify(TextAlign::End)
            .text(n.to_string())
            .build(ui);
    }
}

/// Texto secundário (dicas, estados vazios).
pub fn hint(ui: &mut nuon::Ui, w: f32, h: f32, text: &str) {
    nuon::label()
        .size(w, h)
        .font_size(theme::FONT_M)
        .color(theme::text_muted())
        .text(text)
        .build(ui);
}

/// Etiqueta de atalho de teclado (ex.: "L").
pub fn kbd(ui: &mut nuon::Ui, key: &str) {
    let w = (key.chars().count() as f32 * 9.0 + 12.0).max(22.0);
    nuon::quad()
        .size(w, 22.0)
        .color(theme::surface_sunken())
        .border_color(theme::border_strong())
        .border_width(1.0)
        .border_radius([theme::RADIUS_S; 4])
        .build(ui);
    nuon::label()
        .size(w, 22.0)
        .font_size(theme::FONT_S)
        .color(theme::text_muted())
        .text(key)
        .build(ui);
}

// ── Cartão clicável ─────────────────────────────────────────────────

/// Cartão grande clicável (menu principal). O conteúdo é desenhado pelo
/// chamador dentro de `content`, que recebe o `hover_t` para animar.
pub fn card(
    ui: &mut nuon::Ui,
    id: impl Hash,
    w: f32,
    h: f32,
    content: impl FnOnce(&mut nuon::Ui, f32),
) -> Response {
    let r = interact(ui, wid(id), w, h);
    let t = r.hover_t;
    let lift = -3.0 * t;
    nuon::translate().y(lift).build(ui, |ui| {
        nuon::quad()
            .size(w, h)
            .color(theme::surface().mix(theme::surface_hover(), t))
            .border_color(theme::border().mix(theme::accent_2(), t * 0.7))
            .border_width(1.0)
            .border_radius([theme::RADIUS_L; 4])
            .build(ui);
        content(ui, t);
    });
    r
}
