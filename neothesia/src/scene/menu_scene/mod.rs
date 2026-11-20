mod state;
use bytes::Bytes;
use state::{Page, UiState};

mod midi_picker;
pub(crate) use midi_picker::is_midi_file;
use midi_picker::{ open_midi_file, open_midi_file_picker};

mod neo_btn;
use neo_btn::{neo_btn, neo_btn_icon};

mod library_page;
mod main_page;
mod recordings_page;
mod results_page;
mod song_page;
mod settings;
mod shell;
mod compare_page;
mod tracks;

use std::{collections::HashSet, future::Future, time::Duration};

use crate::utils::{BoxFuture, noop_waker_ref, window::WinitEvent};
use neothesia_core::render::{BgPipeline, ImageIdentifier, QuadRenderer, TextRenderer};

use winit::{
    event::WindowEvent,
    keyboard::{Key, NamedKey},
};

use crate::{NeothesiaEvent, context::Context, icons, locals, scene::Scene, song::Song};
use midi_file::midly::MidiMessage;

use super::NuonRenderer;

/// Ease-out cúbico; `t` é limitado a [0, 1]
fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}

type MsgFn = Box<dyn FnOnce(&mut UiState, &mut Context)>;

fn on_async<T, Fut, FN>(future: Fut, f: FN) -> BoxFuture<MsgFn>
where
    T: 'static,
    Fut: Future<Output = T> + Send + 'static,
    FN: FnOnce(T, &mut UiState, &mut Context) + Send + 'static,
{
    Box::pin(async {
        let res = future.await;
        let f: MsgFn = Box::new(move |data, ctx| f(res, data, ctx));
        f
    })
}

#[derive(Default, Debug)]
struct MidiInputState {
    input_keys: HashSet<u8>,
}

impl MidiInputState {
    fn note_on(&mut self, note: u8) {
        self.input_keys.insert(note);
    }

    fn note_off(&mut self, note: u8) {
        self.input_keys.remove(&note);
    }

    fn is_pressed(&self, note: u8) -> bool {
        self.input_keys.contains(&note)
    }
}

pub struct MenuScene {
    bg_pipeline: BgPipeline,
    text_renderer: TextRenderer,
    nuon_renderer: NuonRenderer,

    logo: ImageIdentifier,

    state: UiState,

    context: std::task::Context<'static>,
    futures: Vec<BoxFuture<MsgFn>>,

    quad_pipeline: QuadRenderer,
    nuon: nuon::Ui,

    midi_input_state: MidiInputState,

    /// Segundos desde que o menu abriu (anima a entrada)
    age: f32,
}

impl MenuScene {
    pub fn new(ctx: &mut Context, song: Option<Song>) -> Self {
        let mut iced_state = UiState::new(ctx, song);
        match crate::utils::capture::env_page().as_deref() {
            Some("library") => iced_state.go_to(Page::Library),
            Some("settings") => iced_state.go_to(Page::Settings),
            Some("tracks") => iced_state.go_to(Page::TrackSelection),
            Some("compare") => {
                // NEOTHESIA_SONG=<música> compara com a melhor gravação real;
                // sem NEOTHESIA_SONG usa a demonstração (test.mid com takes
                // semeados numa pasta temporária)
                let song = match std::env::var_os("NEOTHESIA_SONG")
                    .filter(|s| !s.is_empty())
                {
                    Some(path) => path.into(),
                    None => {
                        crate::recording::demo::seed_takes(ctx);
                        crate::recording::demo::song_path()
                    }
                };
                let takes = ctx.takes.list(&song);
                if let Some(take) = takes
                    .iter()
                    .filter(|t| {
                        crate::scene::menu_scene::song_page::is_complete(t, &takes)
                    })
                    .max_by(|a, b| a.accuracy.total_cmp(&b.accuracy))
                    .or_else(|| takes.first())
                    .cloned()
                {
                    if let Err(e) = iced_state.open_compare(song, take) {
                        log::error!("compare demo/real falhou: {e}");
                    }
                }
            }
            Some("song") => {
                // NEOTHESIA_SONG=/caminho/musica.mid (por omissão, test.mid de demonstração)
                // Com NEOTHESIA_SONG usa as gravações reais; sem ele, a demonstração.
                let song = match std::env::var_os("NEOTHESIA_SONG").filter(|s| !s.is_empty()) {
                    Some(s) => s.into(),
                    None => {
                        crate::recording::demo::seed_takes(ctx);
                        crate::recording::demo::song_path()
                    }
                };
                iced_state.open_song(ctx, song);
            }
            Some("results") => {
                ctx.last_result = Some(crate::recording::demo::session_result());
            }
            Some("recordings") => {
                // NEOTHESIA_RECORDINGS_SONG=/caminho/musica.mid mostra gravações reais
                match std::env::var_os("NEOTHESIA_RECORDINGS_SONG") {
                    Some(song) => iced_state.open_recordings(ctx, song.into()),
                    None => {
                        crate::recording::demo::seed_takes(ctx);
                        iced_state.open_recordings(ctx, crate::recording::demo::song_path());
                    }
                }
            }
            _ => {}
        }

        // Acabou uma sessão no ecrã de tocar: mostrar o resultado
        if let Some(result) = ctx.last_result.take() {
            iced_state.result = Some(result);
            iced_state.go_to(Page::Results);
        }

        let quad_pipeline = ctx.quad_renderer_factory.new_renderer();
        let text_renderer = ctx.text_renderer_factory.new_renderer();

        let mut nuon_renderer = NuonRenderer::new(ctx);

        let logo = Bytes::from_static(include_bytes!("../../../../assets/banner.png"));
        let logo = nuon_renderer.add_image(neothesia_core::render::Image::new(
            &ctx.gpu.device,
            &ctx.gpu.queue,
            logo,
        ));

        Self {
            bg_pipeline: BgPipeline::new(&ctx.gpu),
            text_renderer,
            state: iced_state,
            nuon_renderer,

            logo,

            context: std::task::Context::from_waker(noop_waker_ref()),
            futures: Vec::new(),

            quad_pipeline,
            nuon: {
                let mut ui = nuon::Ui::new();
                if let Some((x, y)) = crate::utils::capture::env_mouse() {
                    ui.mouse_move(x, y);
                }
                ui
            },
            midi_input_state: MidiInputState::default(),
            age: 0.0,
        }
    }

    fn main_ui(&mut self, ctx: &mut Context) {
        if self.state.is_loading() {
            let width = ctx.window_state.logical_size.width;
            let height = ctx.window_state.logical_size.height;

            nuon::label()
                .size(width, height)
                .font_size(30.0)
                .text(locals::menu::loading())
                .text_justify(nuon::TextAlign::Center)
                .build(&mut self.nuon);
            return;
        }

        let mut nuon = std::mem::replace(&mut self.nuon, nuon::Ui::new());

        match self.state.current() {
            Page::Exit => self.exit_page_ui(ctx, &mut nuon),
            Page::Main => self.main_page_ui(ctx, &mut nuon),
            Page::Library => self.library_page_ui(ctx, &mut nuon),
            Page::Settings => self.settings_page_ui(ctx, &mut nuon),
            Page::TrackSelection => self.tracks_page_ui(ctx, &mut nuon),
            Page::Results => self.results_page_ui(ctx, &mut nuon),
            Page::Recordings => self.recordings_page_ui(ctx, &mut nuon),
            Page::Song => self.song_page_ui(ctx, &mut nuon),
            Page::Compare => self.compare_page_ui(ctx, &mut nuon),
        }

        self.nuon = nuon;
    }

    fn exit_page_ui(&mut self, ctx: &mut Context, ui: &mut nuon::Ui) {
        let win_w = ctx.window_state.logical_size.width;
        let win_h = ctx.window_state.logical_size.height;

        let btn_w = 320.0;
        let btn_h = 50.0;
        let btn_gap = 5.0;

        let text_h = 80.0;

        let full_w = btn_w * 2.0 + btn_gap;
        let full_h = btn_h + text_h;

        nuon::translate()
            .x(nuon::center_x(win_w, full_w))
            .y(nuon::center_y(win_h, full_h))
            .build(ui, |ui| {
                nuon::label()
                    .text(locals::menu::exit_question())
                    .font_size(30.0)
                    .size(full_w, text_h)
                    .build(ui);

                nuon::translate().y(text_h).add_to_current(ui);

                if neo_btn()
                    .size(btn_w, btn_h)
                    .label(locals::menu::no())
                    .build(ui)
                {
                    self.state.go_back();
                }

                nuon::translate().x(btn_w).add_to_current(ui);
                nuon::translate().x(btn_gap).add_to_current(ui);

                if neo_btn()
                    .size(btn_w, btn_h)
                    .label(locals::menu::yes())
                    .build(ui)
                {
                    ctx.proxy.send_event(NeothesiaEvent::Exit).ok();
                }
            });
    }
}

impl Scene for MenuScene {
    #[profiling::function]
    fn update(&mut self, ctx: &mut Context, delta: Duration) {
        self.quad_pipeline.clear();
        self.bg_pipeline.update_time(delta);
        self.age += delta.as_secs_f32();
        ctx.library.poll_scan();
        self.state.tick(ctx);

        self.futures
            .retain_mut(|f| match f.as_mut().poll(&mut self.context) {
                std::task::Poll::Ready(msg) => {
                    msg(&mut self.state, ctx);
                    false
                }
                std::task::Poll::Pending => true,
            });

        self.state.tick(ctx);

        self.main_ui(ctx);

        super::render_nuon(&mut self.nuon, &mut self.nuon_renderer, ctx);

        self.text_renderer.update(
            ctx.window_state.physical_size,
            ctx.window_state.scale_factor as f32,
        );
        self.quad_pipeline.prepare();
    }

    #[profiling::function]
    fn render<'pass>(&'pass mut self, rpass: &mut wgpu_jumpstart::RenderPass<'pass>) {
        self.bg_pipeline.render(rpass);
        self.quad_pipeline.render(rpass);
        self.text_renderer.render(rpass);
        self.nuon_renderer.render(rpass);
    }

    fn window_event(&mut self, ctx: &mut Context, event: &WindowEvent) {
        if let WindowEvent::MouseWheel { delta, .. } = event {
            let y = match delta {
                winit::event::MouseScrollDelta::LineDelta(_, y) => y * 60.0,
                winit::event::MouseScrollDelta::PixelDelta(position) => position.y as f32,
            };

            self.nuon.mouse_wheel(y);
        }

        if let WindowEvent::DroppedFile(path) = event {
            if matches!(self.state.current(), Page::Library) {
                if path.is_dir() {
                    ctx.library.add_folder(path.clone());
                } else if is_midi_file(path) {
                    ctx.library.enqueue(path.clone());
                }
            } else if !matches!(self.state.current(), Page::Main) {
                log::warn!("Ignoring dropped file {path:?}, not on the main menu");
            } else if self.state.is_loading() {
                log::warn!("Ignoring dropped file {path:?}, another file is still loading");
            } else if is_midi_file(path) {
                self.futures
                    .push(open_midi_file(&mut self.state, path.clone()));
            } else {
                log::warn!("Ignoring dropped file {path:?}, not a MIDI file");
            }
        }

        if event.cursor_moved() {
            self.nuon.mouse_move(
                ctx.window_state.cursor_logical_position.x,
                ctx.window_state.cursor_logical_position.y,
            );
        } else if event.left_mouse_pressed() {
            self.nuon.mouse_down();
        } else if event.left_mouse_released() {
            self.nuon.mouse_up();
        } else if event.back_mouse_pressed() {
            self.state.go_back();
        }

        match self.state.current() {
            Page::Exit => {
                if event.key_pressed(Key::Named(NamedKey::Enter)) {
                    ctx.proxy.send_event(NeothesiaEvent::Exit).unwrap();
                }

                if event.key_pressed(Key::Named(NamedKey::Escape)) {
                    self.state.go_back();
                }
            }
            Page::Main => {
                if event.key_pressed(Key::Named(NamedKey::Tab)) {
                    self.futures
                        .push(open_midi_file_picker(&mut self.state, ctx));
                }

                if event.key_pressed(Key::Named(NamedKey::Enter)) {
                    state::play(&self.state, ctx)
                }

                if event.key_pressed(Key::Named(NamedKey::Escape)) {
                    self.state.go_back();
                }

                if event.key_pressed(Key::Character("s")) {
                    self.state.go_to(Page::Settings);
                }

                if event.key_pressed(Key::Character("t")) {
                    self.state.go_to(Page::TrackSelection);
                }

                if event.key_pressed(Key::Character("f")) {
                    state::freeplay(&self.state, ctx);
                }

                // G: gravações da última música aberta
                if event.key_pressed(Key::Character("g"))
                    && let Some(song) = ctx.config.last_opened_song().cloned()
                {
                    self.state.open_recordings(ctx, song);
                }
            }
            Page::Library => self.library_page_window_event(ctx, event),
            Page::Results => self.results_page_window_event(ctx, event),
            Page::Recordings => self.recordings_page_window_event(ctx, event),
            Page::Song => self.song_page_window_event(ctx, event),
            Page::Compare => self.compare_page_window_event(ctx, event),
            Page::Settings => {
                if event.key_pressed(Key::Named(NamedKey::Escape)) {
                    self.state.go_back();
                }
            }
            Page::TrackSelection => {
                if event.key_pressed(Key::Named(NamedKey::Enter)) {
                    state::play(&self.state, ctx);
                }

                if event.key_pressed(Key::Named(NamedKey::Escape)) {
                    self.state.go_back();
                }
            }
        }
    }

    fn midi_event(&mut self, ctx: &mut Context, channel: u8, message: &MidiMessage) {
        match message {
            MidiMessage::NoteOn { key, .. } => {
                self.midi_input_state.note_on(key.as_int());
                self.handle_range_detection_noteon(ctx, key.as_int());
            }
            MidiMessage::NoteOff { key, .. } => {
                self.midi_input_state.note_off(key.as_int());
            }
            _ => {}
        }
        ctx.output_manager
            .connection()
            .midi_event(channel.into(), *message);
    }
}
