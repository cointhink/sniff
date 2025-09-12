use std::time::{Duration, Instant};

use alloy_primitives::{U256, utils::format_units};
use crossterm::terminal::disable_raw_mode;
use futures_util::StreamExt;
use reqwest_websocket::Message;
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
    let mut tui = UI::init();
    let (mut tx, mut rx) = ws::connect(&config.geth_url).await.unwrap();

    ws::subscribe(&mut tx, "newPendingTransactions").await;
    ws::subscribe(&mut tx, "newHeads").await;
    let mut timer = timer::Timer::new();

    let mut stop = false;
    let mut one_second = time::interval(Duration::from_secs(1));
    while !stop {
        tokio::select! {
            Some(evt) = tui.reader.next() => {
                let key_str = ui::key_in(evt.unwrap()) ;
                stop = ui::is_key_quit(&key_str);
            },

             _ = one_second.tick() =>  tui.draw(&timer),

            Some(message) = rx.next() => {
                match parse_message(&mut timer, message) {
                    Some(msg) => good_msg(&mut tui, &mut timer, msg),
                    None => (),
             }
            }
        }
    }
    disable_raw_mode().unwrap(); // ratatui::restore()
}

fn good_msg(tui: &mut UI, timer: &mut Timer, msg: RxMsgs) {
    tui.add_msg(msg);
    tui.draw(&timer);
    timer.reset_after_seconds(10);
}

#[derive(serde::Deserialize)]
struct RpcResponse {
    params: Option<RpcParams>,
}
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum RxMsgs {
    UnconfirmedTx(UnconfirmedTx),
    NewHeader(NewHeader),
}
impl RxMsgs {
    fn to_string(self: &Self) -> String {
        match self {
            RxMsgs::UnconfirmedTx(tx) => tx.to_string(),
            RxMsgs::NewHeader(header) => header.to_string(),
        }
    }
}

#[derive(serde::Deserialize)]
struct RpcParams {
    result: RxMsgs,
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
            _ => hex_sig.to_string(),
        }
    } else {
        hex_sig.to_string()
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
        ); // this is a lot for one call to the logger
        let rpc_response = serde_json::from_str::<RpcResponse>(&text)
            .or_else(|err| -> Result<_, String> { panic!("{} {}", err, text) })
            .unwrap(); //RpcResponse
        match rpc_response.params {
            Some(params) => {
                match &params.result {
                    RxMsgs::UnconfirmedTx(_tx) => {}
                    RxMsgs::NewHeader(_header) => {}
                };
                Some(params.result)
            }
            None => None,
        }
    } else {
        None
    }
}
