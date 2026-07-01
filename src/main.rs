use std::time::Duration;

use crossterm::terminal::disable_raw_mode;
use futures_util::{stream::SplitSink, StreamExt};
use reqwest_websocket::{Message, WebSocket};
use timer::Timer;
use tokio::time;
use ui::UI;

use crate::eth::{RpcMsgs, RpcNoticeTypes, RpcResponseTypes, RxMsgs};

mod config;
mod eth;
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

#[derive(Default)]
struct SubState {
    id: String,
    state: bool,
}

#[derive(Default)]
struct State {
    pending_tx_sub: SubState,
    new_heads_sub: SubState,
}

#[tokio::main]
async fn async_main() {
    let config = config::CONFIG.get().unwrap();
    let (mut tx, mut rx) = ws::connect(&config.geth_url).await.unwrap();
    let mut tui = UI::init();

    ws::subscribe(&mut tx, "newPendingTransactions").await;
    ws::subscribe(&mut tx, "newHeads").await;
    let mut timer = timer::Timer::new();
    let mut state = State::default();
    let mut stop = false;
    let mut one_second = time::interval(Duration::from_secs(1));
    while !stop {
        tokio::select! {
            Some(evt) = tui.reader.next() => do_key(&mut stop, evt),
            Some(message) = rx.next() => {
                do_msg(&mut tui, &mut tx, &mut state, &mut timer, message).await;
                tui.draw(&state, &timer)
             },
            _ = one_second.tick() =>  tui.draw(&state, &timer),
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
    state: &mut State,
    timer: &mut Timer,
    message: Result<Message, reqwest_websocket::Error>,
) {
    match parse_message(timer, message) {
        Some(msg) => {
            log::info!("parsing {:?}", msg.to_string());
            match msg {
                RxMsgs::TxId(id) => ws::get_tx_by_hash(tx, &id).await,
                RxMsgs::SubscriptionResult(sub) => {
                    if sub.id == state.pending_tx_sub.id {
                        state.pending_tx_sub.state = true;
                    }
                }
                _ => good_msg(tui, timer, msg),
            }
        }
        None => (),
    }
}

fn good_msg(tui: &mut UI, timer: &mut Timer, msg: RxMsgs) {
    tui.add_msg(msg);
    timer.reset_after_seconds(10);
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
                    RpcNoticeTypes::SubscriptionResult(subscription_result) => {
                        Some(RxMsgs::SubscriptionResult(subscription_result))
                    }
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
