//! Gravações do que o utilizador toca e comparação com o original.
//! Ver `ARQUITETURA.md` › 7.

pub mod compare;
pub mod demo;
pub mod recorder;
pub mod store;

use std::time::Duration;

pub use compare::{ExpectedNote, Report, SectionStats, compare};
pub use recorder::TakeRecorder;
pub use store::{Take, TakeStore};

/// Uma nota tocada pelo utilizador, no tempo da música (sem o lead-in).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayedNote {
    pub note: u8,
    pub start: Duration,
    pub end: Duration,
    pub velocity: u8,
}

/// Resultado da última sessão no ecrã de tocar, entregue ao menu
/// através de `Context::last_result`.
#[derive(Debug, Clone)]
pub struct SessionResult {
    pub take: Take,
    pub report: Report,
}
