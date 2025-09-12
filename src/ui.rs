use crossterm::event::{Event, EventStream, KeyCode, KeyModifiers};
use ratatui::DefaultTerminal;

pub struct UI {
    pub terminal: DefaultTerminal,
    pub reader: EventStream,
}

impl UI {
    pub fn init() -> Self {
        let terminal = ratatui::init();
        let reader = EventStream::new();
        UI { terminal, reader }
    }
}

pub fn key_in(event: Event) -> String {
    let mut keystring = String::new();
    match event {
        Event::Key(key) => {
            if key.modifiers == KeyModifiers::CONTROL {
                keystring.insert_str(0, "^")
            }
            match key.code {
                KeyCode::Char(char) => {
                    keystring.push(char);
                }
                _ => (),
            }
        }
        _ => (),
    }
    keystring
}

pub fn key_quit(key_str: &str) -> bool {
    key_str == "q" || key_str == "^c"
}
