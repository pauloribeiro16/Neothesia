use std::{
    cell::{OnceCell, RefCell},
    rc::Rc,
    sync::Arc,
};

pub use cosmic_text::FontSystem;

/// Família do texto normal. No macOS é a SF Pro ("System Font"); se não existir,
/// o cosmic-text usa a Roboto embutida.
pub const DEFAULT_FAMILY: &str = "System Font";

thread_local! {
     static FONT_SYSTEM: OnceCell<Rc<RefCell<FontSystem>>> = const { OnceCell::new() };
}

/// Returns the global [`FontSystem`].
pub fn font_system() -> Rc<RefCell<FontSystem>> {
    FONT_SYSTEM.with(|system| {
        system
            .get_or_init(|| {
                let mut fs = FontSystem::new_with_fonts([
                    cosmic_text::fontdb::Source::Binary(Arc::new(include_bytes!(
                        "../../assets/fonts/Iced-Icons.ttf"
                    ))),
                    cosmic_text::fontdb::Source::Binary(Arc::new(include_bytes!(
                        "../../assets/fonts/Roboto-Regular.ttf"
                    ))),
                    cosmic_text::fontdb::Source::Binary(Arc::new(include_bytes!(
                        "../../assets/fonts/bootstrap-icons.ttf"
                    ))),
                    cosmic_text::fontdb::Source::Binary(Arc::new(include_bytes!(
                        "../../assets/fonts/Leland.otf"
                    ))),
                    cosmic_text::fontdb::Source::Binary(Arc::new(include_bytes!(
                        "../../assets/fonts/neothesia-icons.ttf"
                    ))),
                ]);
                // macOS: serifa "New York" (títulos) e "System Font" (SF Pro, texto),
                // lidas do sistema — nada é redistribuído. Noutros sistemas, ou se
                // faltarem, o texto cai para a Roboto embutida.
                for path in [
                    "/System/Library/Fonts/NewYork.ttf",
                    "/System/Library/Fonts/SFNS.ttf",
                ] {
                    if std::path::Path::new(path).exists() {
                        let _ = fs.db_mut().load_font_file(path);
                    }
                }
                Rc::new(RefCell::new(fs))
            })
            .clone()
    })
}
