use futures_util::{SinkExt, TryStreamExt};
use reqwest_websocket::Message;

mod config;
mod rpc;
mod ws;

#[tokio::main]
async fn main() {
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();
    let config: config::Config = config::read_type(config::FILENAME);
    log::info!("scoop {}", config.geth_url);
    let (mut tx, mut rx) = ws::connect(&config.geth_url).await.unwrap();
    let rpc_subscribe = rpc::RpcCall::new(
        "eth_subscribe",
        vec![
            serde_json::Value::String("newPendingTransactions".into()),
            serde_json::Value::Bool(true),
        ],
    );
    let sender = async move {
        let cmd = serde_json::to_string(&rpc_subscribe).unwrap();
        log::info!("{}", cmd);
        tx.send(Message::Text(cmd)).await.unwrap();
    };

    let reader = async move {
        let mut rx_byte_count: u128 = 0;
        let mut rx_msg_count: u128 = 0;
        let mut now = std::time::Instant::now();
        while let Some(message) = rx.try_next().await.unwrap() {
            if let Message::Text(text) = message {
                rx_byte_count += text.len() as u128;
                rx_msg_count += 1;
                // println!("{}", text);
            }
            let duration = now.elapsed();
            let duration_ms10 = now.elapsed().as_millis() + 1;
            log::info!(
                "elapsed {:?}. {:?} msg/sec. {:?} kbytes/sec",
                duration,
                (rx_msg_count * 1000).div_ceil(duration_ms10) as f64 / 10.0,
                (rx_byte_count).div_ceil(duration_ms10) as f64 / 10.0
            );
            if duration.as_secs() > 10 {
                now = std::time::Instant::now();
                rx_byte_count = 0;
            }
        }
    };
    futures_util::future::join(sender, reader).await;
}
