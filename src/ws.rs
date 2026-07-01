use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use reqwest_websocket::{Error, Message, RequestBuilderExt, WebSocket};

use crate::rpc::{self, RpcCall};

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

pub async fn subscribe(tx: &mut SplitSink<WebSocket, Message>, topic: &str) -> String {
    let topic = serde_json::Value::String(topic.to_string());
    let full_tx = serde_json::Value::Bool(true);
    let mut params = vec![&topic];
    if topic == "newPendingTransactions" {
        params.push(&full_tx);
    };
    let id = RpcCall::new_id();
    let rpc_sub_json = rpc::call(&id, "eth_subscribe", params);
    tx.send(Message::Text(rpc_sub_json)).await.unwrap();
    id
}

//params: ["0x88df016429689c079f3b2f6ad39fa052532c56795b733da78a91ebe6a713944b"]
pub async fn get_tx_by_hash(tx: &mut SplitSink<WebSocket, Message>, hash: &str) {
    let hash_str = serde_json::Value::String(hash.to_owned());
    let params = vec![&hash_str];
    let id = RpcCall::new_id();
    let rpc_json = rpc::call(&id, "eth_getTransactionByHash", params);
    tx.send(Message::Text(rpc_json)).await.unwrap();
}
