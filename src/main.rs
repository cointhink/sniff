use std::time::Duration;

use crossterm::event::{Event, EventStream, KeyCode, KeyModifiers};
use futures_util::{SinkExt, StreamExt, stream::SplitSink};
use ratatui::{Frame, style::Stylize, text};
use reqwest_websocket::{Message, WebSocket};
use tokio::{sync::mpsc, time};

mod config;
mod rpc;
mod timer;
mod ws;

fn main() {
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();
    config::setup(config::FILENAME);
    log::info!("scoop loaded {}", config::path(config::FILENAME));

    async_main();
}

#[tokio::main]
async fn async_main() {
    let mut terminal = ratatui::init();
    let (tui_tx, mut tui_rx) = mpsc::channel::<String>(10);
    let mut reader = EventStream::new();

    let mut interval = time::interval(Duration::from_secs(1));

    let config = config::CONFIG.get().unwrap();
    let (mut tx, mut rx) = ws::connect(&config.geth_url).await.unwrap();

    subscribe(&mut tx, "newPendingTransactions").await;
    let mut timer = timer::Timer::new();

    let mut stop = false;
    while !stop {
        tokio::select! {
            Some(evt) = reader.next() => {
              let key =key_in(evt.unwrap()) ;
                if key == "q" || key == "^c" {
                    stop = true
                }
            }

             _ = interval.tick() => { terminal.draw(|frame| render(frame)); },

            Some(message) = rx.next() => {
               if let Message::Text(text) = message.unwrap() {
                  timer.next_msg(text.len());
                   // println!("{}", text);
               }
             timer.report();
             timer.reset_after_seconds(10);
            }
        }
    }
    ratatui::restore()
}

fn render(frame: &mut Frame) {
    let title = text::Line::from("Ratatui async example").centered().bold();
    let title_area = frame.area();
    frame.render_widget(title, title_area);
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
