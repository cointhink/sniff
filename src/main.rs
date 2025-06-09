use futures_util::{SinkExt, TryStreamExt};
use reqwest_websocket::Message;

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
    let sender = async move {
        let rpc_subscribe = rpc::RpcCall::new(
            "eth_subscribe",
            vec![
                serde_json::Value::String("newPendingTransactions".into()),
                serde_json::Value::Bool(true),
            ],
        );
        let cmd = serde_json::to_string(&rpc_subscribe).unwrap();
        log::info!("{}", cmd);
        tx.send(Message::Text(cmd)).await.unwrap();
    };

    let reader = async move {
        let mut timer = timer::new();

        while let Some(message) = rx.try_next().await.unwrap() {
            if let Message::Text(text) = message {
                timer.next_msg(text.len());
                // println!("{}", text);
            }
            timer.report();
            timer.reset_after_seconds(10);
        }
    };

    futures_util::future::join(sender, reader).await;
}
