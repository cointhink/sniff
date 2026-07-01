use std::{
    sync::{Arc, RwLock, RwLockReadGuard},
    time::Instant,
};

use crossterm::event::{Event, EventStream, KeyCode, KeyModifiers};
use ratatui::{
    layout::{Constraint, Layout},
    style::Stylize,
    text,
    widgets::{Row, Table},
    DefaultTerminal, Frame,
};
use reqwest::Url;

use crate::{config, timer, RxMsgs, State};

pub struct UI {
    pub terminal: DefaultTerminal,
    pub reader: EventStream,
    pub state: StateList,
}

type StateItem = (Instant, RxMsgs);
type StateList = Arc<RwLock<Vec<StateItem>>>;

impl UI {
    pub fn init() -> Self {
        let terminal = ratatui::init();
        let reader = EventStream::new();
        let state = StateList::default();

        UI {
            terminal,
            reader,
            state,
        }
    }

    pub fn draw(self: &mut Self, state: &State, timer: &timer::Timer) {
        let items = self.state.read().unwrap();
        self.terminal
            .draw(|frame| render(items, frame, state, timer))
            .unwrap();
    }

    pub fn add_msg(self: &mut Self, msg: RxMsgs) {
        let mut rows = self.state.write().unwrap();
        rows.push((Instant::now(), msg));
    }
}

fn render(
    items: RwLockReadGuard<Vec<StateItem>>,
    frame: &mut Frame,
    state: &State,
    timer: &timer::Timer,
) {
    let config = config::CONFIG.get().unwrap();
    let times = timer.report();
    let title = text::Line::from(format!(
        "{} unconfirmed eth transactions. {} kb/sec {} msg/sec",
        items.len(),
        times.0,
        times.1
    ))
    .centered()
    .bold();
    let headers = text::Line::from(format!(
        "{:5} {:42} {:42} {:7} {:5} {:8}",
        "age", "to", "from", "eth", "gas", "call"
    ));
    let vertical = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Fill(1),
    ]);
    let [title_area, header_area, body_area] = vertical.areas(frame.area());
    let title_layout = Layout::horizontal([
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
    ]);
    let [title_host_area, title_title_area, title_lights_area] = title_layout.areas(title_area);

    let last_bit = items.len().saturating_sub(body_area.height as usize);
    let rows = items[last_bit..]
        .iter()
        .map(|item| {
            Row::new(vec![format!(
                "{:.3} {}",
                Instant::now().duration_since(item.0).as_millis() as f64 / 1000.0,
                item.1.to_string()
            )])
        })
        .collect::<Vec<Row>>();

    let widths = [Constraint::Max(body_area.width)];
    let table = Table::new(rows, widths);

    let host = Url::parse(&config.geth_url).unwrap();
    frame.render_widget(host.host_str(), title_host_area);
    frame.render_widget(title, title_title_area);
    let state_line = format!(
        "{} {}",
        if state.pending_tx_sub.state {
            "[NewTx]"
        } else {
            "[]"
        },
        if state.new_heads_sub.state {
            "[NewHead]"
        } else {
            "[]"
        },
    );
    frame.render_widget(state_line, title_lights_area);
    frame.render_widget(headers, header_area);
    frame.render_widget(table, body_area);
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

pub fn is_key_quit(key_str: &str) -> bool {
    key_str == "q" || key_str == "^c"
}
