//! Junta eventos NoteOn/NoteOff em notas (`PlayedNote`).

use std::{collections::HashMap, time::Duration};

use midi_file::midly::MidiMessage;

use super::PlayedNote;

#[derive(Debug, Default)]
pub struct TakeRecorder {
    /// nota -> (início, velocidade)
    active: HashMap<u8, (Duration, u8)>,
    notes: Vec<PlayedNote>,
    last_time: Duration,
}

impl TakeRecorder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.notes.is_empty() && self.active.is_empty()
    }

    pub fn note_count(&self) -> usize {
        self.notes.len() + self.active.len()
    }

    pub fn note_on(&mut self, note: u8, velocity: u8, t: Duration) {
        // Um novo ataque na mesma tecla fecha a nota anterior
        self.note_off(note, t);
        self.active.insert(note, (t, velocity));
        self.last_time = self.last_time.max(t);
    }

    pub fn note_off(&mut self, note: u8, t: Duration) {
        if let Some((start, velocity)) = self.active.remove(&note) {
            self.notes.push(PlayedNote {
                note,
                start,
                end: t.max(start),
                velocity,
            });
        }
        self.last_time = self.last_time.max(t);
    }

    /// Trata uma mensagem MIDI (NoteOn com velocidade 0 conta como NoteOff).
    pub fn midi_event(&mut self, message: &MidiMessage, t: Duration) {
        match *message {
            MidiMessage::NoteOn { key, vel } if vel.as_int() > 0 => {
                self.note_on(key.as_int(), vel.as_int(), t)
            }
            MidiMessage::NoteOn { key, .. } | MidiMessage::NoteOff { key, .. } => {
                self.note_off(key.as_int(), t)
            }
            _ => {}
        }
    }

    /// Fecha as notas ainda pressionadas e devolve todas, ordenadas pelo início.
    pub fn finish(mut self, t: Duration) -> Vec<PlayedNote> {
        let t = t.max(self.last_time);
        let mut active: Vec<_> = self.active.drain().collect();
        active.sort_unstable_by_key(|(n, _)| *n);
        for (note, (start, velocity)) in active {
            self.notes.push(PlayedNote {
                note,
                start,
                end: t.max(start),
                velocity,
            });
        }
        self.notes.sort_by_key(|n| (n.start, n.note));
        self.notes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(v: u64) -> Duration {
        Duration::from_millis(v)
    }

    #[test]
    fn pairs_on_and_off() {
        let mut r = TakeRecorder::new();
        r.note_on(60, 100, ms(0));
        r.note_on(64, 90, ms(10));
        r.note_off(60, ms(500));
        let notes = r.finish(ms(800));
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0], PlayedNote { note: 60, start: ms(0), end: ms(500), velocity: 100 });
        // 64 ficou pressionada: fechada no fim
        assert_eq!(notes[1].end, ms(800));
    }

    #[test]
    fn retrigger_closes_previous_note() {
        let mut r = TakeRecorder::new();
        r.note_on(60, 100, ms(0));
        r.note_on(60, 100, ms(300));
        let notes = r.finish(ms(400));
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].end, ms(300));
    }

    #[test]
    fn note_on_velocity_zero_is_note_off() {
        let mut r = TakeRecorder::new();
        r.midi_event(&MidiMessage::NoteOn { key: 60.into(), vel: 80.into() }, ms(0));
        r.midi_event(&MidiMessage::NoteOn { key: 60.into(), vel: 0.into() }, ms(200));
        let notes = r.finish(ms(1000));
        assert_eq!(notes[0].end, ms(200));
    }
}
