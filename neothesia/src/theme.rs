//! Tema visual partilhado. Toda a UI usa estas cores, medidas e helpers,
//! para a app ficar consistente (ver `.agents/README.md`).

#![allow(dead_code)]

use std::hash::Hash;

use nuon::Color;

// ── Cores: direção "sala de concerto" (ver a maqueta aprovada) ─────────
// Fundo escuro e calmo, texto marfim, UM acento dourado para a ação principal.
// Nos dados: azul = certo / a tempo, laranja = a melhorar (distinguíveis por
// daltónicos e em luminosidade).

/// Fundo da app (#0F0E13)
pub fn background() -> Color {
    Color::new_u8(15, 14, 19, 1.0)
}
/// Painéis e cartões (#17161D translúcido)
pub fn surface() -> Color {
    Color::new_u8(23, 22, 29, 0.88)
}
pub fn surface_hover() -> Color {
    Color::new_u8(33, 31, 41, 0.94)
}
/// Item escolhido / em destaque (tom quente)
pub fn surface_active() -> Color {
    Color::new_u8(52, 43, 30, 0.92)
}
pub fn surface_sunken() -> Color {
    Color::new_u8(13, 12, 17, 0.75)
}
pub fn border() -> Color {
    Color::new_u8(244, 239, 230, 0.08)
}
pub fn border_strong() -> Color {
    Color::new_u8(244, 239, 230, 0.18)
}
/// Dourado — ação principal, seleção, a tua melhor marca (#E3B35C)
pub fn accent() -> Color {
    Color::new_u8(227, 179, 92, 1.0)
}
/// Dourado claro — hover/foco do acento (#F0C979)
pub fn accent_2() -> Color {
    Color::new_u8(240, 201, 121, 1.0)
}
/// Texto sobre o dourado
pub fn on_accent() -> Color {
    Color::new_u8(15, 14, 19, 1.0)
}
/// Texto principal — marfim (#F4EFE6)
pub fn text() -> Color {
    Color::new_u8(244, 239, 230, 1.0)
}
/// Texto secundário (#A8A2B3)
pub fn text_muted() -> Color {
    Color::new_u8(168, 162, 179, 1.0)
}
/// Texto terciário / rótulos (#7D7889)
pub fn text_faint() -> Color {
    Color::new_u8(125, 120, 137, 1.0)
}
/// Dados: certo / a tempo (#6EA8FE)
pub fn good() -> Color {
    Color::new_u8(110, 168, 254, 1.0)
}
/// Dados: a melhorar (#F08A4B)
pub fn warn() -> Color {
    Color::new_u8(240, 138, 75, 1.0)
}
/// Ações destrutivas (apagar)
pub fn danger() -> Color {
    Color::new_u8(240, 98, 98, 1.0)
}

/// Família da serifa dos títulos (macOS: "New York"; noutros sistemas cai na
/// família de texto). Usar em títulos de página e de cartões grandes.
pub const FONT_DISPLAY: &str = ".New York";

// ── Medidas ─────────────────────────────────────────────────────────
pub const RADIUS_S: f32 = 6.0;
pub const RADIUS_M: f32 = 10.0;
pub const RADIUS_L: f32 = 16.0;

pub const SPACE_XS: f32 = 4.0;
pub const SPACE_S: f32 = 8.0;
pub const SPACE_M: f32 = 12.0;
pub const SPACE_L: f32 = 16.0;
pub const SPACE_XL: f32 = 24.0;
pub const SPACE_XXL: f32 = 32.0;

pub const FONT_S: f32 = 13.0;
pub const FONT_M: f32 = 16.0;
pub const FONT_L: f32 = 20.0;
pub const FONT_XL: f32 = 28.0;
pub const FONT_XXL: f32 = 40.0;

pub const ROW_H: f32 = 48.0;
pub const BUTTON_H: f32 = 44.0;

// ── Animação ────────────────────────────────────────────────────────

/// Ease-out cúbico; `t` é limitado a [0, 1]
pub fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}

/// Valor animado guardado no `Ui` por `id`: aproxima-se suavemente de `target`
/// a cada frame. Útil para hover, seleção, abrir/fechar.
pub fn anim(ui: &mut nuon::Ui, id: impl Hash, target: f32, speed: f32) -> f32 {
    let key = nuon::Id::hash_with(|h| {
        "theme_anim".hash(h);
        id.hash(h);
    });
    let v: &mut f32 = ui.data(key);
    *v += (target - *v) * speed.clamp(0.0, 1.0);
    if (target - *v).abs() < 0.002 {
        *v = target;
    }
    *v
}

// ── Desenho ─────────────────────────────────────────────────────────

/// Cartão/painel com fundo, borda subtil e cantos arredondados.
pub fn panel(ui: &mut nuon::Ui, w: f32, h: f32) {
    nuon::quad()
        .size(w, h)
        .color(surface())
        .border_color(border())
        .border_width(1.0)
        .border_radius([RADIUS_L; 4])
        .build(ui);
}

/// Texto truncado com reticências para caber em `w` px (aproximação por largura média).
pub fn fit_text(s: &str, w: f32, font_size: f32) -> String {
    let max = (w / (font_size * 0.56)).max(4.0) as usize;
    if s.chars().count() <= max {
        s.to_owned()
    } else {
        let cut: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{}…", cut.trim_end())
    }
}
