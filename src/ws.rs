use futures_util::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use reqwest_websocket::{Error, Message, RequestBuilderExt, WebSocket};

use crate::rpc;

pub async fn connect(
    url: &str,
) -> Result<(SplitSink<WebSocket, Message>, SplitStream<WebSocket>), Error> {
    match reqwest::Client::default().get(url).upgrade().send().await {
        Ok(socket) => match socket.into_websocket().await {
            Ok(ws) => Ok(ws.split()),
            Err(e) => Err(e),
        },
        Err(e) => Err(e),
    }
}

pub async fn subscribe(tx: &mut SplitSink<WebSocket, Message>, topic: &str) {
    let topic = serde_json::Value::String(topic.to_string());
    let full_tx = serde_json::Value::Bool(true);
    let mut params = vec![&topic];
    if topic == "newPendingTransactions" {
        params.push(&full_tx);
    };
    let rpc_sub_json = rpc::call("eth_subscribe", params);
    log::info!("{}", rpc_sub_json);
    tx.send(Message::Text(rpc_sub_json)).await.unwrap();
}
