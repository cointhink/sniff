use futures_util::{
    StreamExt,
    stream::{SplitSink, SplitStream},
};
use reqwest_websocket::{Error, Message, RequestBuilderExt, WebSocket};

pub async fn connect(
    url: &str,
) -> Result<(SplitSink<WebSocket, Message>, SplitStream<WebSocket>), Error> {
    let websocket = reqwest::Client::default()
        .get(url)
        .upgrade()
        .send()
        .await?
        .into_websocket()
        .await?;

    Ok(websocket.split())
}
