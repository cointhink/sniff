use std::{
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use alloy_primitives::utils::format_units;
use crossterm::event::{Event, EventStream, KeyCode, KeyModifiers};
use futures_util::{SinkExt, StreamExt, stream::SplitSink};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::Stylize,
    text::{self},
    widgets::{Row, Table},
};
use reqwest_websocket::{Message, WebSocket};
use timer::Timer;
use tokio::time;

mod config;
mod rpc;
mod timer;
mod ws;

type AppStateItem = (Instant, UnconfirmedTx);

fn main() {
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();
    config::setup(config::FILENAME);
    log::info!("scoop loaded {}", config::path(config::FILENAME));

    async_main();
}

#[tokio::main]
async fn async_main() {
    let mut terminal = ratatui::init();
    let mut reader = EventStream::new();

    let mut interval = time::interval(Duration::from_secs(1));

    let config = config::CONFIG.get().unwrap();
    let (mut tx, mut rx) = ws::connect(&config.geth_url).await.unwrap();

    subscribe(&mut tx, "newPendingTransactions").await;
    let mut timer = timer::Timer::new();

    let ui_state = Arc::<RwLock<Vec<AppStateItem>>>::default();

    let mut stop = false;
    while !stop {
        tokio::select! {
            Some(evt) = reader.next() => {
              let key =key_in(evt.unwrap()) ;
                if key == "q" || key == "^c" {
                    stop = true
                }
            }

             _ = interval.tick() => {
                 terminal.draw(|frame| render(frame, &ui_state, &timer)).unwrap();
             },

            Some(message) = rx.next() => {
             select_message(&mut timer,&ui_state, message);
             terminal.draw(|frame| render(frame, &ui_state, &timer)).unwrap();
             timer.reset_after_seconds(10);
            }
        }
    }
    ratatui::restore()
}

#[derive(serde::Deserialize)]
struct RpcResponse {
    params: Option<RpcParams>,
}
#[derive(serde::Deserialize)]
struct RpcParams {
    result: UnconfirmedTx,
}
#[derive(serde::Deserialize)]
struct UnconfirmedTx {
    from: String,
    to: Option<String>,
    value: String,
    input: String,
}
impl UnconfirmedTx {
    fn to_string(self: &Self) -> String {
        let value_wei = u128::from_str_radix(&self.value[2..], 16).unwrap();
        format!(
            "{:42} {:42} {:5} {:8}",
            self.from,
            self.to.clone().unwrap_or("- contract-creation".to_string()),
            format_units(value_wei, 18).unwrap()[0..5].to_string(),
            if self.input.len() > 2 {
                self.input[..8].to_string()
            } else {
                "".to_string()
            },
        )
    }
}

fn select_message(
    timer: &mut Timer,
    ui_state: &Arc<RwLock<Vec<AppStateItem>>>,
    message: Result<Message, reqwest_websocket::Error>,
) {
    if let Message::Text(text) = message.unwrap() {
        timer.next_msg(text.len());
        let rpc_response = serde_json::from_str::<RpcResponse>(&text)
            .or_else(|_| -> Result<_, String> { panic!("{}", text) })
            .unwrap(); //RpcResponse
        match rpc_response.params {
            Some(params) => {
                let mut rows = ui_state.write().unwrap();
                rows.push((Instant::now(), params.result));
            }
            None => (),
        }
    }
}

fn render(frame: &mut Frame, state: &Arc<RwLock<Vec<AppStateItem>>>, timer: &timer::Timer) {
    let items = state.read().unwrap();
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
        "{:5} {:42} {:42} {:5} {:8}",
        "age", "to", "from", "eth", "call"
    ));
    let vertical = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Fill(1),
    ]);
    let [title_area, header_area, body_area] = vertical.areas(frame.area());

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

    frame.render_widget(title, title_area);
    frame.render_widget(headers, header_area);
    frame.render_widget(table, body_area);
}

fn key_in(event: Event) -> String {
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

pub async fn subscribe(tx: &mut SplitSink<WebSocket, Message>, topic: &str) {
    let rpc_sub_json = rpc::call(
        "eth_subscribe",
        vec![
            &serde_json::Value::String(topic.into()),
            &serde_json::Value::Bool(true),
        ],
    );
    log::info!("{}", rpc_sub_json);
    tx.send(Message::Text(rpc_sub_json)).await.unwrap();
}
