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
                theme::on_accent(),
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

    // Lupa desenhada (o glifo ⌕ não existe na fonte): anel + pega
    let (cx, cy, r) = (theme::SPACE_M + 9.0, h / 2.0 - 2.0, 6.0);
    nuon::quad()
        .pos(cx - r, cy - r)
        .size(r * 2.0, r * 2.0)
        .border_color(theme::text_faint())
        .border_width(1.8)
        .border_radius([r; 4])
        .build(ui);
    for k in 0..4 {
        let d = r * 0.75 + k as f32 * 1.4;
        nuon::quad()
            .pos(cx + d - 1.2, cy + d - 1.2)
            .size(2.6, 2.6)
            .color(theme::text_faint())
            .border_radius([1.3; 4])
            .build(ui);
    }

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

/// Título de página (serifa, grande). Ex.: "Biblioteca".
pub fn page_title(ui: &mut nuon::Ui, w: f32, text: &str) {
    nuon::label()
        .size(w, 52.0)
        .font_size(theme::FONT_XXL)
        .font_family(theme::FONT_DISPLAY)
        .color(theme::text())
        .text_justify(TextAlign::Start)
        .text(text)
        .build(ui);
}

/// Rótulo pequeno em maiúsculas (ex.: "A ESTUDAR", "AUTORES").
pub fn eyebrow(ui: &mut nuon::Ui, w: f32, text: &str) {
    nuon::label()
        .size(w, 18.0)
        .font_size(theme::FONT_S - 1.0)
        .color(theme::text_muted())
        .text_justify(TextAlign::Start)
        .text(text.to_uppercase())
        .build(ui);
}

/// Título de secção (ex.: "Playlist", "Recentes"), com `count` opcional.
pub fn section_title(ui: &mut nuon::Ui, w: f32, text: &str, count: Option<usize>) {
    nuon::label()
        .size(w, 28.0)
        .font_size(theme::FONT_L)
        .font_family(theme::FONT_DISPLAY)
        .color(theme::text())
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

// ── Gráficos ────────────────────────────────────────────────────────

/// Gráfico de barras verticais (ex.: erros por compasso). Escala ao máximo
/// de `values`. As barras crescem com animação ao aparecer. Devolve o índice
/// da barra sob o rato, se houver (para tooltips ou clique).
pub fn bar_chart(
    ui: &mut nuon::Ui,
    id: impl Hash,
    w: f32,
    h: f32,
    values: &[f32],
    color: Color,
) -> Option<usize> {
    let id = wid(id);
    let grow = theme::ease_out(theme::anim(ui, ("bar_grow", id.as_raw()), 1.0, 0.06));

    nuon::quad()
        .size(w, h)
        .color(theme::surface_sunken())
        .border_radius([theme::RADIUS_M; 4])
        .build(ui);

    if values.is_empty() {
        return None;
    }

    let pad = theme::SPACE_S;
    let inner_w = w - pad * 2.0;
    let inner_h = h - pad * 2.0;
    let max = values.iter().cloned().fold(0.0_f32, f32::max).max(1.0);
    let slot = inner_w / values.len() as f32;
    let gap = (slot * 0.25).clamp(1.0, 4.0);
    let bar_w = (slot - gap).max(1.0);

    let mut hovered = None;
    for (i, v) in values.iter().enumerate() {
        let x = pad + i as f32 * slot;
        let ev = nuon::click_area(wid((id.as_raw(), "bar", i)))
            .pos(x, pad)
            .size(slot, inner_h)
            .build(ui);
        let is_hover = ev.is_hovered();
        if is_hover {
            hovered = Some(i);
        }

        let bh = (v / max * inner_h * grow).max(if *v > 0.0 { 2.0 } else { 0.0 });
        if bh <= 0.0 {
            // linha de base fina para compassos sem valor
            nuon::quad()
                .pos(x, pad + inner_h - 1.0)
                .size(bar_w, 1.0)
                .color(theme::border_strong())
                .build(ui);
            continue;
        }
        nuon::quad()
            .pos(x, pad + inner_h - bh)
            .size(bar_w, bh)
            .color(if is_hover { theme::accent_2() } else { color })
            .border_radius([2.0, 2.0, 0.0, 0.0])
            .build(ui);
    }
    hovered
}

/// Gráfico de linha com pontos (ex.: precisão ao longo das gravações).
/// `values` em 0..1, desenhados da esquerda (mais antigo) para a direita.
/// A escala ajusta-se aos valores (com margem), para se verem variações
/// pequenas (ex.: 93 % → 96 %). Devolve o índice do ponto sob o rato, se houver.
pub fn line_chart(ui: &mut nuon::Ui, id: impl Hash, w: f32, h: f32, values: &[f32]) -> Option<usize> {
    let (lo, hi) = auto_range(values);
    let scaled: Vec<f32> = values.iter().map(|v| (v - lo) / (hi - lo)).collect();
    line_chart_scaled(ui, id, w, h, &scaled)
}

/// Intervalo a mostrar para `values`: mínimo e máximo com 25 % de margem,
/// pelo menos 10 pontos percentuais de amplitude, dentro de 0..1.
pub fn auto_range(values: &[f32]) -> (f32, f32) {
    if values.is_empty() {
        return (0.0, 1.0);
    }
    let min = values.iter().cloned().fold(f32::MAX, f32::min);
    let max = values.iter().cloned().fold(f32::MIN, f32::max);
    let span = (max - min).max(0.10);
    let mid = (max + min) / 2.0;
    let lo = (mid - span * 0.75).max(0.0);
    let hi = (lo + span * 1.5).min(1.0);
    let lo = (hi - span * 1.5).max(0.0);
    (lo, hi)
}

/// Como `line_chart`, mas com `values` já normalizados em 0..1 (sem auto-escala).
pub fn line_chart_scaled(ui: &mut nuon::Ui, id: impl Hash, w: f32, h: f32, values: &[f32]) -> Option<usize> {
    let id = wid(id);
    let grow = theme::ease_out(theme::anim(ui, ("line_grow", id.as_raw()), 1.0, 0.06));

    nuon::quad()
        .size(w, h)
        .color(theme::surface_sunken())
        .border_radius([theme::RADIUS_M; 4])
        .build(ui);

    let pad = theme::SPACE_L;
    let inner_w = w - pad * 2.0;
    let inner_h = h - pad * 2.0;

    // Linhas de referência (topo, meio, base)
    for frac in [0.0_f32, 0.5, 1.0] {
        nuon::quad()
            .pos(pad, pad + inner_h * (1.0 - frac))
            .size(inner_w, 1.0)
            .color(theme::border())
            .build(ui);
    }

    if values.is_empty() {
        return None;
    }

    let point = |i: usize, v: f32| -> (f32, f32) {
        let x = if values.len() == 1 {
            pad + inner_w / 2.0
        } else {
            pad + inner_w * i as f32 / (values.len() - 1) as f32
        };
        let y = pad + inner_h * (1.0 - v.clamp(0.0, 1.0) * grow);
        (x, y)
    };

    // Segmentos: aproximados por pequenos quadrados ao longo da linha
    for i in 1..values.len() {
        let (x0, y0) = point(i - 1, values[i - 1]);
        let (x1, y1) = point(i, values[i]);
        let len = ((x1 - x0).powi(2) + (y1 - y0).powi(2)).sqrt();
        let steps = (len / 2.0).ceil().max(1.0) as usize;
        for s in 0..=steps {
            let t = s as f32 / steps as f32;
            nuon::quad()
                .pos(x0 + (x1 - x0) * t - 1.0, y0 + (y1 - y0) * t - 1.0)
                .size(2.5, 2.5)
                .color(theme::accent())
                .border_radius([1.25; 4])
                .build(ui);
        }
    }

    let mut hovered = None;
    for (i, v) in values.iter().enumerate() {
        let (x, y) = point(i, *v);
        let ev = nuon::click_area(wid((id.as_raw(), "pt", i)))
            .pos(x - 10.0, y - 10.0)
            .size(20.0, 20.0)
            .build(ui);
        let r = if ev.is_hovered() {
            hovered = Some(i);
            6.0
        } else {
            4.0
        };
        nuon::quad()
            .pos(x - r, y - r)
            .size(r * 2.0, r * 2.0)
            .color(if ev.is_hovered() { theme::accent_2() } else { theme::text() })
            .border_color(theme::accent())
            .border_width(2.0)
            .border_radius([r; 4])
            .build(ui);
    }
    hovered
}
