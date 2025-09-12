use crossterm::event::EventStream;
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
