use std::sync::LazyLock;

static LOCAL: LazyLock<String> = LazyLock::new(|| {
    let locale = sys_locale::get_locale().unwrap_or_else(|| "en-US".to_string());
    // sys_locale says that this is suposed to be a BCP 47 tag, so I supose this split is fine
    // eg. "pl-PL" -> "pl"
    locale.split('-').next().unwrap_or("en").to_lowercase()
});

pub mod menu {
    use super::LOCAL;

    pub fn exit_question() -> String {
        match LOCAL.as_str() {
            "pl" => "Czy chcesz wyjść?".into(),
            "de" => "Möchtest Du die App verlassen?".into(),
            "pt" => "Queres sair?".into(),
            _ => "Do you want to exit?".into(),
        }
    }

    pub fn no() -> String {
        match LOCAL.as_str() {
            "pl" => "Nie".into(),
            "de" => "Nein".into(),
            "pt" => "Não".into(),
            _ => "No".into(),
        }
    }

    pub fn yes() -> String {
        match LOCAL.as_str() {
            "pl" => "Tak".into(),
            "de" => "Ja".into(),
            "pt" => "Sim".into(),
            _ => "Yes".into(),
        }
    }

    pub fn loading() -> String {
        match LOCAL.as_str() {
            "pl" => "Ładowanie...".into(),
            "de" => "Wird geladen...".into(),
            "pt" => "A carregar...".into(),
            _ => "Loading...".into(),
        }
    }

    pub fn select_file() -> String {
        match LOCAL.as_str() {
            "pl" => "Wybierz plik".into(),
            "de" => "Datei auswählen".into(),
            "pt" => "Selecionar ficheiro".into(),
            _ => "Select File".into(),
        }
    }

    pub fn settings() -> String {
        match LOCAL.as_str() {
            "pl" => "Ustawienia".into(),
            "de" => "Einstellungen".into(),
            "pt" => "Definições".into(),
            _ => "Settings".into(),
        }
    }

    pub fn exit() -> String {
        match LOCAL.as_str() {
            "pl" => "Wyjdź".into(),
            "de" => "Beenden".into(),
            "pt" => "Sair".into(),
            _ => "Exit".into(),
        }
    }

    pub fn freeplay() -> String {
        match LOCAL.as_str() {
            "pl" => "Swobodna gra".into(),
            "de" => "Freies Spiel".into(),
            "pt" => "Tocar livre".into(),
            _ => "FreePlay".into(),
        }
    }

    pub fn play() -> String {
        match LOCAL.as_str() {
            "pl" => "Graj".into(),
            "de" => "Abspielen".into(),
            "pt" => "Tocar".into(),
            _ => "Play".into(),
        }
    }

    pub fn tracks() -> String {
        match LOCAL.as_str() {
            "pl" => "Ścieżki".into(),
            "de" => "Stücke".into(),
            "pt" => "Faixas".into(),
            _ => "Tracks".into(),
        }
    }

    pub fn subtitle() -> String {
        match LOCAL.as_str() {
            "pl" => "Twój fortepian, twoje utwory".into(),
            "de" => "Dein Klavier, deine Stücke".into(),
            "pt" => "O teu piano, as tuas músicas".into(),
            _ => "Your piano, your music".into(),
        }
    }

    pub fn continue_card() -> String {
        match LOCAL.as_str() {
            "pl" => "Kontynuuj".into(),
            "de" => "Fortsetzen".into(),
            "pt" => "Continuar".into(),
            _ => "Continue".into(),
        }
    }

    pub fn open_file() -> String {
        match LOCAL.as_str() {
            "pl" => "Otwórz plik".into(),
            "de" => "Datei öffnen".into(),
            "pt" => "Abrir ficheiro".into(),
            _ => "Open file".into(),
        }
    }
}

pub mod freeplay {
    use super::LOCAL;
    use std::path::Path;

    pub fn no_note_events_recorded() -> String {
        match LOCAL.as_str() {
            "pl" => "Nie nagrano żadnych nut".into(),
            "de" => "Keine Notenereignisse aufgezeichnet".into(),
            "pt" => "Nenhuma nota gravada".into(),
            _ => "No note events recorded".into(),
        }
    }

    pub fn failed_to_write_midi_file() -> String {
        match LOCAL.as_str() {
            "pl" => "Nie udało się zapisać pliku MIDI".into(),
            "de" => "MIDI-Datei konnte nicht geschrieben werden".into(),
            "pt" => "Falha ao escrever o ficheiro MIDI".into(),
            _ => "Failed to write MIDI file".into(),
        }
    }

    pub fn recording(seconds: f32) -> String {
        match LOCAL.as_str() {
            "pl" => format!("Nagrywanie {seconds:.1}s"),
            "de" => format!("Aufnahme läuft ({seconds:.1} s)"),
            "pt" => format!("A gravar {seconds:.1}s"),
            _ => format!("Recording {seconds:.1}s"),
        }
    }

    pub fn recorded(seconds: f32) -> String {
        match LOCAL.as_str() {
            "pl" => format!("Nagrano {seconds:.1}s"),
            "de" => format!("Aufnahme abgeschlossen ({seconds:.1} s)"),
            "pt" => format!("Gravado {seconds:.1}s"),
            _ => format!("Recorded {seconds:.1}s"),
        }
    }

    pub fn saved_recording_to(path: &Path) -> String {
        match LOCAL.as_str() {
            "pl" => format!("Zapisano nagranie w {}", path.display()),
            "de" => format!("Aufnahme gespeichert unter: {}", path.display()),
            "pt" => format!("Gravação guardada em {}", path.display()),
            _ => format!("Saved recording to {}", path.display()),
        }
    }
}

pub mod settings {
    use super::LOCAL;

    pub fn output() -> String {
        match LOCAL.as_str() {
            "pl" => "Wyjście".into(),
            "de" => "Ausgang".into(),
            "pt" => "Saída".into(),
            _ => "Output".into(),
        }
    }

    pub fn input() -> String {
        match LOCAL.as_str() {
            "pl" => "Wejście".into(),
            "de" => "Eingang".into(),
            "pt" => "Entrada".into(),
            _ => "Input".into(),
        }
    }

    pub fn note_range() -> String {
        match LOCAL.as_str() {
            "pl" => "Zakres nut".into(),
            "de" => "Notenumfang".into(),
            "pt" => "Gama de notas".into(),
            _ => "Note Range".into(),
        }
    }

    pub fn keyboard_size() -> String {
        match LOCAL.as_str() {
            "pl" => "Rozmiar klawiatury".into(),
            "de" => "Tastaturgröße".into(),
            "pt" => "Tamanho do teclado".into(),
            _ => "Keyboard Size".into(),
        }
    }

    pub fn keys(count: u8) -> String {
        match LOCAL.as_str() {
            "pl" => format!("{count} klawiszy"),
            "de" => format!("{count} Tasten"),
            "pt" => format!("{count} teclas"),
            _ => format!("{count} Keys"),
        }
    }

    pub fn custom() -> String {
        match LOCAL.as_str() {
            "pl" => "Własny".into(),
            "de" => "Eigene".into(),
            "pt" => "Personalizado".into(),
            _ => "Custom".into(),
        }
    }

    pub fn range_start() -> String {
        match LOCAL.as_str() {
            "pl" => "Początek".into(),
            "de" => "Start".into(),
            "pt" => "Início".into(),
            _ => "Start".into(),
        }
    }

    pub fn range_end() -> String {
        match LOCAL.as_str() {
            "pl" => "Koniec".into(),
            "de" => "Ende".into(),
            "pt" => "Fim".into(),
            _ => "End".into(),
        }
    }

    pub fn waterfall() -> String {
        match LOCAL.as_str() {
            "pl" => "Wodospad".into(),
            "de" => "Wasserfall".into(),
            "pt" => "Cascata".into(),
            _ => "Waterfall".into(),
        }
    }

    pub fn keyboard() -> String {
        match LOCAL.as_str() {
            "pl" => "Klawiatura".into(),
            "de" => "Tastatur".into(),
            "pt" => "Teclado".into(),
            _ => "Keyboard".into(),
        }
    }

    pub fn octave_lines() -> String {
        match LOCAL.as_str() {
            "pl" => "Linie oktaw".into(),
            "de" => "Oktavlinien".into(),
            "pt" => "Linhas de oitava".into(),
            _ => "Octave Lines".into(),
        }
    }

    pub fn octave_lines_subtitle() -> String {
        match LOCAL.as_str() {
            "pl" => "Pionowe linie przy każdym C i F, ułatwiające odnalezienie klawiszy".into(),
            "de" => "Vertikale Linien bei jedem C und F zur Orientierung".into(),
            "pt" => "Linhas verticais em cada C e F para ajudar a encontrar as teclas".into(),
            _ => "Vertical lines at every C and F to help you find keys".into(),
        }
    }

    pub fn bar_lines() -> String {
        match LOCAL.as_str() {
            "pl" => "Linie taktów".into(),
            "de" => "Taktstriche".into(),
            "pt" => "Linhas de compasso".into(),
            _ => "Bar Lines".into(),
        }
    }

    pub fn bar_lines_subtitle() -> String {
        match LOCAL.as_str() {
            "pl" => "Pozioma linia na początku każdego taktu".into(),
            "de" => "Horizontale Linie am Anfang jedes Takts".into(),
            "pt" => "Linha horizontal no início de cada compasso".into(),
            _ => "Horizontal line at the start of each measure".into(),
        }
    }

    pub fn note_names() -> String {
        match LOCAL.as_str() {
            "pl" => "Nazwy nut".into(),
            "de" => "Notennamen".into(),
            "pt" => "Nomes das notas".into(),
            _ => "Note Names".into(),
        }
    }

    pub fn note_names_subtitle() -> String {
        match LOCAL.as_str() {
            "pl" => "Wyświetlaj nazwę nuty na każdej opadającej nucie".into(),
            "de" => "Notennamen auf jeder fallenden Note anzeigen".into(),
            "pt" => "Mostrar o nome da nota em cada nota a cair".into(),
            _ => "Show the note name on each falling note".into(),
        }
    }

    pub fn naming_style() -> String {
        match LOCAL.as_str() {
            "pl" => "Styl nazw".into(),
            "de" => "Benennungsstil".into(),
            "pt" => "Estilo dos nomes".into(),
            _ => "Naming Style".into(),
        }
    }

    pub fn naming_style_keyboard_subtitle() -> String {
        match LOCAL.as_str() {
            "pl" => "Wybierz sposób nazywania klawiszy".into(),
            "de" => "Wähle, wie die Tasten beschriftet werden".into(),
            "pt" => "Escolhe como nomear as teclas".into(),
            _ => "Choose how the keys are labeled".into(),
        }
    }

    pub fn naming_style_waterfall_subtitle() -> String {
        match LOCAL.as_str() {
            "pl" => "Wybierz sposób nazywania opadających nut".into(),
            "de" => "Wähle, wie fallende Noten beschriftet werden".into(),
            "pt" => "Escolhe como nomear as notas a cair".into(),
            _ => "Choose how falling notes are labeled".into(),
        }
    }

    pub fn note_naming_octaves() -> String {
        match LOCAL.as_str() {
            "pl" => "Oktawy".into(),
            "de" => "Oktaven".into(),
            "pt" => "Oitavas".into(),
            _ => "Octaves".into(),
        }
    }

    pub fn note_naming_letters() -> String {
        match LOCAL.as_str() {
            "pl" => "Angielskie nazwy nut (C, C#, D)".into(),
            "de" => "Englische Notennamen (C, C#, D)".into(),
            "pt" => "Nomes ingleses (C, C#, D)".into(),
            _ => "English Note Names (C, C#, D)".into(),
        }
    }

    pub fn note_naming_solfege() -> String {
        match LOCAL.as_str() {
            "pl" => "Solfeż (Do, Di, Mi)".into(),
            "de" => "Solfège (Do, Di, Mi)".into(),
            "pt" => "Solfejo (Do, Di, Mi)".into(),
            _ => "Solfege (Do, Di, Mi)".into(),
        }
    }

    pub fn note_naming_latin() -> String {
        match LOCAL.as_str() {
            "pl" => "Łacińskie (Do, Re, Mi)".into(),
            "de" => "Lateinisch (Do, Re, Mi)".into(),
            "pt" => "Latina (Do, Re, Mi)".into(),
            _ => "Latin (Do, Re, Mi)".into(),
        }
    }

    pub fn key_glow() -> String {
        match LOCAL.as_str() {
            "pl" => "Poświata klawiszy".into(),
            "de" => "Tastenleuchten".into(),
            "pt" => "Brilho das teclas".into(),
            _ => "Key Glow".into(),
        }
    }

    pub fn key_glow_subtitle() -> String {
        match LOCAL.as_str() {
            "pl" => "Podświetlaj klawisze podczas gry".into(),
            "de" => "Tasten beim Spielen aufleuchten lassen".into(),
            "pt" => "Iluminar as teclas ao serem tocadas".into(),
            _ => "Light up keys as they are played".into(),
        }
    }

    pub fn chord_name() -> String {
        match LOCAL.as_str() {
            "pl" => "Nazwa akordu".into(),
            "de" => "Akkordname".into(),
            "pt" => "Nome do acorde".into(),
            _ => "Chord Name".into(),
        }
    }

    pub fn chord_name_subtitle() -> String {
        match LOCAL.as_str() {
            "pl" => "Wyświetlaj nazwę granego akordu nad klawiszami".into(),
            "de" => "Gespielten Akkord über den Tasten anzeigen".into(),
            "pt" => "Mostrar o nome do acorde tocado por cima das teclas".into(),
            _ => "Name the chord you are playing above the keys".into(),
        }
    }

    pub fn soundfont() -> String {
        match LOCAL.as_str() {
            "pl" => "SoundFont".into(),
            "de" => "SoundFont".into(),
            "pt" => "SoundFont".into(),
            _ => "SoundFont".into(),
        }
    }

    pub fn select_file() -> String {
        match LOCAL.as_str() {
            "pl" => "Wybierz plik".into(),
            "de" => "Datei auswählen".into(),
            "pt" => "Selecionar ficheiro".into(),
            _ => "Select File".into(),
        }
    }

    pub fn audio_gain() -> String {
        match LOCAL.as_str() {
            "pl" => "Wzmocnienie dźwięku".into(),
            "de" => "Audio-Verstärkung".into(),
            "pt" => "Volume do áudio".into(),
            _ => "Audio Gain".into(),
        }
    }

    pub fn separate_channels() -> String {
        match LOCAL.as_str() {
            "pl" => "Oddzielne kanały".into(),
            "de" => "Kanäle trennen".into(),
            "pt" => "Canais separados".into(),
            _ => "Separate Channels".into(),
        }
    }

    pub fn separate_channels_subtitle() -> String {
        match LOCAL.as_str() {
            "pl" => "Przypisz osobny kanał MIDI do każdej ścieżki".into(),
            "de" => "Jeder Spur einen eigenen MIDI-Kanal zuweisen".into(),
            "pt" => "Atribuir um canal MIDI diferente a cada faixa".into(),
            _ => "Assign different MIDI channel to each track".into(),
        }
    }

    pub fn auto_detect() -> String {
        match LOCAL.as_str() {
            "pl" => "Automatyczne wykrywanie".into(),
            "de" => "Automatische Erkennung".into(),
            "pt" => "Deteção automática".into(),
            _ => "Auto-detect".into(),
        }
    }

    pub fn auto_detect_subtitle() -> String {
        match LOCAL.as_str() {
            "pl" => "Automatycznie wykryj zakres podłączonej klawiatury".into(),
            "de" => "Tonumfang vom angeschlossenen Keyboard automatisch erkennen".into(),
            "pt" => "Detetar automaticamente a gama do teclado ligado".into(),
            _ => "Auto-detect range from connected keyboard".into(),
        }
    }

    pub fn detect() -> String {
        match LOCAL.as_str() {
            "pl" => "Wykryj".into(),
            "de" => "Erkennen".into(),
            "pt" => "Detetar".into(),
            _ => "Detect".into(),
        }
    }

    pub fn detection_subtitle() -> String {
        match LOCAL.as_str() {
            "pl" => "Naciśnij skrajnie lewy i skrajnie prawy klawisz na klawiaturze...".into(),
            "de" => {
                "Spiele die ganz linke und die ganz rechts liegende Taste auf Deinem Keyboard..."
                    .into()
            }
            "pt" => "Toca a tecla mais à esquerda e a mais à direita do teclado...".into(),
            _ => "Play the far-left and far-right key on your keyboard...".into(),
        }
    }

    pub fn cancel() -> String {
        match LOCAL.as_str() {
            "pl" => "Anuluj".into(),
            "de" => "Abbrechen".into(),
            "pt" => "Cancelar".into(),
            _ => "Cancel".into(),
        }
    }

    pub fn detection_in_progress(step: usize, total: usize) -> String {
        match LOCAL.as_str() {
            "pl" => format!("Wykrywanie w toku... Krok {step}/{total}"),
            "de" => format!("Erkennung läuft... Schritt {step}/{total}"),
            "pt" => format!("Deteção em curso... Passo {step}/{total}"),
            _ => format!("Detection in progress... Step {step}/{total}"),
        }
    }
}

pub mod tracks {
    use super::LOCAL;

    pub fn percussion() -> String {
        match LOCAL.as_str() {
            "pl" => "Perkusja".into(),
            "de" => "Schlagzeug".into(),
            "pt" => "Percussão".into(),
            _ => "Percussion".into(),
        }
    }

    pub fn notes(count: usize) -> String {
        match LOCAL.as_str() {
            "pl" => format!("{count} nut"),
            "de" => format!("{count} Noten"),
            "pt" => format!("{count} notas"),
            _ => format!("{count} notes"),
        }
    }

    pub fn auto() -> String {
        match LOCAL.as_str() {
            "pl" => "Auto".into(),
            "de" => "Auto".into(),
            "pt" => "Auto".into(),
            _ => "Auto".into(),
        }
    }

    pub fn human() -> String {
        match LOCAL.as_str() {
            "pl" => "Człowiek".into(),
            "de" => "Mensch".into(),
            "pt" => "Humano".into(),
            _ => "Human".into(),
        }
    }

    pub fn mute() -> String {
        match LOCAL.as_str() {
            "pl" => "Wycisz".into(),
            "de" => "Stumm".into(),
            "pt" => "Mudo".into(),
            _ => "Mute".into(),
        }
    }
}

pub mod library {
    use super::LOCAL;

    pub fn title() -> String {
        match LOCAL.as_str() {
            "pl" => "Biblioteka".into(),
            "de" => "Bibliothek".into(),
            "pt" => "Biblioteca".into(),
            _ => "Library".into(),
        }
    }

    pub fn one_song() -> String {
        match LOCAL.as_str() {
            "pl" => "1 utwór".into(),
            "de" => "1 Stück".into(),
            "pt" => "1 música".into(),
            _ => "1 song".into(),
        }
    }

    pub fn songs_count() -> String {
        match LOCAL.as_str() {
            "pl" => "utworów".into(),
            "de" => "Stücke".into(),
            "pt" => "músicas".into(),
            _ => "songs".into(),
        }
    }

    pub fn recent() -> String {
        match LOCAL.as_str() {
            "pl" => "Ostatnie".into(),
            "de" => "Zuletzt".into(),
            "pt" => "Recentes".into(),
            _ => "Recent".into(),
        }
    }

    pub fn musics() -> String {
        match LOCAL.as_str() {
            "pl" => "Utwory".into(),
            "de" => "Stücke".into(),
            "pt" => "Músicas".into(),
            _ => "Songs".into(),
        }
    }

    pub fn playlist() -> String {
        match LOCAL.as_str() {
            "pl" => "Playlista".into(),
            "de" => "Wiedergabeliste".into(),
            "pt" => "Playlist".into(),
            _ => "Playlist".into(),
        }
    }

    pub fn add_folder() -> String {
        match LOCAL.as_str() {
            "pl" => "+ Folder".into(),
            "de" => "+ Ordner".into(),
            "pt" => "+ Pasta".into(),
            _ => "+ Folder".into(),
        }
    }

    pub fn add_folder_action() -> String {
        match LOCAL.as_str() {
            "pl" => "+ Dodaj folder".into(),
            "de" => "+ Ordner hinzufügen".into(),
            "pt" => "+ Adicionar pasta".into(),
            _ => "+ Add folder".into(),
        }
    }

    pub fn refresh() -> String {
        match LOCAL.as_str() {
            "pl" => "Odśwież".into(),
            "de" => "Aktualisieren".into(),
            "pt" => "Atualizar".into(),
            _ => "Refresh".into(),
        }
    }

    pub fn clear() -> String {
        match LOCAL.as_str() {
            "pl" => "Wyczyść".into(),
            "de" => "Leeren".into(),
            "pt" => "Limpar".into(),
            _ => "Clear".into(),
        }
    }

    pub fn play_playlist() -> String {
        match LOCAL.as_str() {
            "pl" => "Odtwórz playlistę".into(),
            "de" => "Wiedergabeliste abspielen".into(),
            "pt" => "Tocar playlist".into(),
            _ => "Play playlist".into(),
        }
    }

    pub fn search_placeholder() -> String {
        match LOCAL.as_str() {
            "pl" => "Pisz, aby wyszukiwać…".into(),
            "de" => "Zum Suchen schreiben…".into(),
            "pt" => "Escreve para pesquisar…".into(),
            _ => "Type to search…".into(),
        }
    }

    pub fn empty_hint() -> String {
        match LOCAL.as_str() {
            "pl" => "Biblioteka jest pusta — dodaj folder z plikami MIDI lub przeciągnij go tutaj".into(),
            "de" => "Bibliothek ist leer — füge einen Ordner mit MIDI-Dateien hinzu oder ziehe ihn hierher".into(),
            "pt" => "Biblioteca vazia — adiciona uma pasta com músicas (.mid) ou arrasta-a para aqui".into(),
            _ => "Library is empty — add a folder with songs (.mid) or drag one here".into(),
        }
    }

    pub fn no_results() -> String {
        match LOCAL.as_str() {
            "pl" => "Nenhum utwór odpowiada wyszukiwaniu".into(),
            "de" => "Keine Stücke entsprechen der Suche".into(),
            "pt" => "Nenhuma música corresponde à pesquisa".into(),
            _ => "No songs match your search".into(),
        }
    }

    pub fn queue_empty_hint() -> String {
        match LOCAL.as_str() {
            "pl" => "Użyj przycisku +, aby dodać utwory".into(),
            "de" => "Verwende die Taste +, um Stücke hinzuzufügen".into(),
            "pt" => "Usa o botão + para juntar músicas".into(),
            _ => "Use the + button to add songs".into(),
        }
    }

    pub fn favorite_add() -> String {
        match LOCAL.as_str() {
            "pl" => "Dodaj do ulubionych".into(),
            "de" => "Zu Favoriten hinzufügen".into(),
            "pt" => "Adicionar aos favoritos".into(),
            _ => "Add to favorites".into(),
        }
    }

    pub fn favorite_remove() -> String {
        match LOCAL.as_str() {
            "pl" => "Usuń z ulubionych".into(),
            "de" => "Aus Favoriten entfernen".into(),
            "pt" => "Remover dos favoritos".into(),
            _ => "Remove from favorites".into(),
        }
    }

    pub fn play_now() -> String {
        match LOCAL.as_str() {
            "pl" => "Odtwórz teraz".into(),
            "de" => "Jetzt abspielen".into(),
            "pt" => "Tocar agora".into(),
            _ => "Play now".into(),
        }
    }

    pub fn enqueue() -> String {
        match LOCAL.as_str() {
            "pl" => "Dodaj do playlisty".into(),
            "de" => "Zur Wiedergabeliste hinzufügen".into(),
            "pt" => "Juntar à playlist".into(),
            _ => "Add to playlist".into(),
        }
    }

    pub fn queue_up() -> String {
        match LOCAL.as_str() {
            "pl" => "Przenieś w górę".into(),
            "de" => "Nach oben verschieben".into(),
            "pt" => "Mover para cima".into(),
            _ => "Move up".into(),
        }
    }

    pub fn queue_down() -> String {
        match LOCAL.as_str() {
            "pl" => "Przenieś w dół".into(),
            "de" => "Nach unten verschieben".into(),
            "pt" => "Mover para baixo".into(),
            _ => "Move down".into(),
        }
    }

    pub fn remove() -> String {
        match LOCAL.as_str() {
            "pl" => "Usuń".into(),
            "de" => "Entfernen".into(),
            "pt" => "Remover".into(),
            _ => "Remove".into(),
        }
    }

    pub fn remove_folder() -> String {
        match LOCAL.as_str() {
            "pl" => "Usuń folder".into(),
            "de" => "Ordner entfernen".into(),
            "pt" => "Remover pasta".into(),
            _ => "Remove folder".into(),
        }
    }
}

pub mod playing {
    use super::LOCAL;

    pub fn speed(speed: f32) -> String {
        match LOCAL.as_str() {
            "pl" => format!("Prędkość: {speed}"),
            "de" => format!("Geschwindigkeit: {speed}"),
            "pt" => format!("Velocidade: {speed}"),
            _ => format!("Speed: {speed}"),
        }
    }

    pub fn animation_speed(speed: f32) -> String {
        match LOCAL.as_str() {
            "pl" => format!("Prędkość animacji: {speed}"),
            "de" => format!("Animationsgeschwindigkeit: {speed}"),
            "pt" => format!("Velocidade da animação: {speed}"),
            _ => format!("Animation Speed: {speed}"),
        }
    }

    pub fn offset(offset: f32) -> String {
        match LOCAL.as_str() {
            "pl" => format!("Przesunięcie: {offset}"),
            "de" => format!("Offset: {offset}"),
            "pt" => format!("Desfasamento: {offset}"),
            _ => format!("Offset: {offset}"),
        }
    }
}
