use std::time::Duration;

use alloy_primitives::{U256, utils::format_units};
use crossterm::terminal::disable_raw_mode;
use futures_util::{StreamExt, stream::SplitSink};
use reqwest_websocket::{Message, WebSocket};
use timer::Timer;
use tokio::time;
use ui::UI;

mod config;
mod rpc;
mod timer;
mod ui;
mod ws;

fn main() {
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();
    config::setup(config::FILENAME);
    log::info!("scoop loaded {}", config::path(config::FILENAME));

    async_main();
}

#[tokio::main]
async fn async_main() {
    let config = config::CONFIG.get().unwrap();
    let (mut tx, mut rx) = ws::connect(&config.geth_url).await.unwrap();
    let mut tui = UI::init();

    ws::subscribe(&mut tx, "newPendingTransactions").await;
    ws::subscribe(&mut tx, "newHeads").await;
    let mut timer = timer::Timer::new();

    let mut stop = false;
    let mut one_second = time::interval(Duration::from_secs(1));
    while !stop {
        tokio::select! {
            Some(evt) = tui.reader.next() => do_key(&mut stop, evt),
            Some(message) = rx.next() => do_msg(&mut tui, &mut tx, &mut timer, message).await,
            _ = one_second.tick() =>  tui.draw(&timer),
        }
    }
    disable_raw_mode().unwrap(); // ratatui::restore()
}

fn do_key(stop: &mut bool, evt: Result<crossterm::event::Event, std::io::Error>) {
    let key_str = ui::key_in(evt.unwrap());
    *stop = ui::is_key_quit(&key_str);
}

async fn do_msg(
    tui: &mut UI,
    tx: &mut SplitSink<WebSocket, Message>,
    timer: &mut Timer,
    message: Result<Message, reqwest_websocket::Error>,
) {
    match parse_message(timer, message) {
        Some(msg) => {
            log::info!("parsing {:?}", msg.to_string());
            match msg {
                RxMsgs::TxId(id) => ws::get_tx_by_hash(tx, &id).await,
                _ => good_msg(tui, timer, msg),
            }
        }
        None => (),
    }
}

fn good_msg(tui: &mut UI, timer: &mut Timer, msg: RxMsgs) {
    tui.add_msg(msg);
    tui.draw(&timer);
    timer.reset_after_seconds(10);
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum RpcMsgs {
    RpcNotice(RpcNotice),
    RpcResponse(RpcResponse),
}

#[derive(serde::Deserialize)]
struct RpcNotice {
    method: String,
    params: RpcNoticeParams,
}

#[derive(serde::Deserialize)]
struct RpcNoticeParams {
    subscription: String,
    result: RpcNoticeTypes,
}
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum RpcNoticeTypes {
    SubscriptionResult(SubscriptionResult),
    BlockHeader(NewHeader),
    TxId(String),
}

#[derive(serde::Deserialize)]
struct RpcResponse {
    id: String,
    result: Option<RpcResponseTypes>,
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum RpcResponseTypes {
    UnconfirmedTx(UnconfirmedTx),
    BlockHeader(NewHeader),
    SubscriptionId(String),
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum RxMsgs {
    SubscriptionResult(SubscriptionResult),
    UnconfirmedTx(UnconfirmedTx),
    BlockHeader(NewHeader),
    TxId(String),
}

impl RxMsgs {
    fn to_string(self: &Self) -> String {
        match self {
            RxMsgs::UnconfirmedTx(tx) => tx.to_string(),
            RxMsgs::BlockHeader(header) => header.to_string(),
            RxMsgs::TxId(id) => format!("txid: {}", id.to_owned()),
            RxMsgs::SubscriptionResult(_subscription_result) => "sub success".to_owned(),
        }
    }
}

#[derive(serde::Deserialize)]
struct NewHeader {
    number: String,
}
impl NewHeader {
    fn to_string(self: &Self) -> String {
        format!("Block {}", self.number())
    }
    fn number(self: &Self) -> U256 {
        U256::from_str_radix(&self.number[2..], 16).unwrap()
    }
}
#[derive(serde::Deserialize)]
struct SubscriptionResult {
    subscription: String,
    result: String,
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
            "{:42} {:42} {:6} {:8}",
            self.from,
            self.to.clone().unwrap_or("- contract-creation".to_string()),
            format_units(value_wei, 18).unwrap()[0..6].to_string(),
            match_fn_signature(&self.input),
        )
    }
}

fn match_fn_signature(hex_sig: &str) -> String {
    // U256::from_be_slice(&hex::decode(hex_sig[8..40].to_string()).unwrap());
    if hex_sig.len() >= 10 {
        match &hex_sig[0..10] {
            "0xa9059cbb" => {
                // erc20 transfer(address,uint256)
                let units = U256::from_str_radix(&hex_sig[74..138], 16).unwrap();
                format!("erc20 xfer {}", units)
            }
            _ => format!("unknown sig: {}", hex_sig.to_string()),
        }
    } else {
        format!("eth transfer")
    }
}

#[cfg(test)]
#[test]
fn test_match_fn_signature() {
    use alloy_primitives::hex;

    let selector = "0xa9059cbb";
    let param1 = hex::encode::<[u8; 32]>(U256::from(1).to_be_bytes());
    let param2 = hex::encode::<[u8; 32]>(U256::from(10_u128.pow(18)).to_be_bytes());
    assert_eq!(
        match_fn_signature(&format!("{}{}{}", selector, param1, param2)),
        "erc20 xfer 1.000"
    );
}

fn parse_message(
    timer: &mut Timer,
    message: Result<Message, reqwest_websocket::Error>,
) -> Option<RxMsgs> {
    if let Message::Text(text) = message.unwrap() {
        timer.next_msg(text.len());
        log::logger().log(
            &log::Record::builder()
                .target("http")
                .args(format_args!("{}", text))
                .build(),
        );
        log::info!("in: {}", text);
        let rpc_response = serde_json::from_str::<RpcMsgs>(&text)
            .or_else(|err| -> Result<_, String> { panic!("{} {}", err, text) })
            .unwrap(); //RpcResponse
        match rpc_response {
            RpcMsgs::RpcNotice(notice) => {
                log::info!("checking response type");
                match notice.params.result {
                    RpcNoticeTypes::TxId(tx) => Some(RxMsgs::TxId(tx)),
                    RpcNoticeTypes::BlockHeader(header) => Some(RxMsgs::BlockHeader(header)),
                    RpcNoticeTypes::SubscriptionResult(_subscription_result) => None,
                }
            }
            RpcMsgs::RpcResponse(response) => match response.result {
                Some(result) => match result {
                    RpcResponseTypes::UnconfirmedTx(tx) => Some(RxMsgs::UnconfirmedTx(tx)),
                    RpcResponseTypes::BlockHeader(_header) => todo!(),
                    RpcResponseTypes::SubscriptionId(_tx_id) => None,
                },
                None => None,
            },
        }
    } else {
        None
    }
}
