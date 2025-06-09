use futures_util::{SinkExt, TryStreamExt, stream::SplitSink};
use reqwest_websocket::{Message, WebSocket};

mod config;
mod rpc;
mod timer;
mod ws;

#[tokio::main]
async fn main() {
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();
    let config = config::setup(config::FILENAME);
    log::info!("scoop loaded {}", config::path(config::FILENAME));

    let (mut tx, mut rx) = ws::connect(&config.geth_url).await.unwrap();

    subscribe(&mut tx, "newPendingTransactions").await;
    let mut timer = timer::Timer::new();

    while let Some(message) = rx.try_next().await.unwrap() {
        if let Message::Text(text) = message {
            timer.next_msg(text.len());
            // println!("{}", text);
        }
        timer.report();
        timer.reset_after_seconds(10);
    }
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
