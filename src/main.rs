use futures_util::{
    SinkExt, StreamExt, TryStreamExt,
    stream::{SplitSink, SplitStream},
};
use reqwest::Client;
use reqwest_websocket::{Error, Message, RequestBuilderExt, WebSocket};
use serde::Serialize;

mod config;

#[derive(Serialize)]
struct RpcCall<'a> {
    jsonrpc: &'static str,
    id: String,
    method: &'a str,
    params: Vec<serde_json::Value>,
}

impl<'a> RpcCall<'a> {
    fn new(method: &'a str, params: Vec<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0",
            id: Self::new_id(),
            method,
            params,
        }
    }

    fn new_id() -> String {
        const ID_LEN: usize = 4;
        let mut buf: [u8; ID_LEN] = [0; ID_LEN];
        for idx in 0..ID_LEN {
            buf[idx] = 97 + fastrand::u8(0..(ID_LEN as u8));
        }
        String::from_utf8(buf.to_vec()).unwrap()
    }
}

#[tokio::main]
async fn main() {
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();
    let config: config::Config = config::read_type(config::FILENAME);
    log::info!("scoop {}", config.geth_url);
    let (mut tx, mut rx) = wsgo(&config.geth_url).await.unwrap();
    let rpc_subscribe = RpcCall::new(
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
        while let Some(message) = rx.try_next().await.unwrap() {
            if let Message::Text(text) = message {
                println!("{}", text);
            }
        }
    };
    futures_util::future::join(sender, reader).await;
}

async fn wsgo(url: &str) -> Result<(SplitSink<WebSocket, Message>, SplitStream<WebSocket>), Error> {
    let websocket = Client::default()
        .get(url)
        .upgrade()
        .send()
        .await?
        .into_websocket()
        .await?;

    Ok(websocket.split())
}
