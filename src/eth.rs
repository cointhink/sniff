use alloy_primitives::{utils::format_units, U256};

#[derive(serde::Deserialize)]
#[serde(untagged)]
pub(crate) enum RpcMsgs {
    RpcNotice(RpcNotice),
    RpcResponse(RpcResponse),
}

#[derive(serde::Deserialize)]
pub(crate) struct RpcNotice {
    pub(crate) method: String,
    pub(crate) params: RpcNoticeParams,
}

#[derive(serde::Deserialize)]
pub(crate) struct RpcNoticeParams {
    pub(crate) subscription: String,
    pub(crate) result: RpcNoticeTypes,
}
#[derive(serde::Deserialize)]
#[serde(untagged)]
pub(crate) enum RpcNoticeTypes {
    SubscriptionResult(SubscriptionResult),
    BlockHeader(NewHeader),
    TxId(String),
}

#[derive(serde::Deserialize)]
pub(crate) struct RpcResponse {
    pub(crate) id: String,
    pub(crate) result: Option<RpcResponseTypes>,
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
pub(crate) enum RpcResponseTypes {
    UnconfirmedTx(UnconfirmedTx),
    BlockHeader(NewHeader),
    SubscriptionId(String),
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
pub(crate) enum RxMsgs {
    SubscriptionResult(SubscriptionResult),
    UnconfirmedTx(UnconfirmedTx),
    BlockHeader(NewHeader),
    TxId(String),
}

impl RxMsgs {
    pub(crate) fn to_string(self: &Self) -> String {
        match self {
            RxMsgs::UnconfirmedTx(tx) => tx.to_string(),
            RxMsgs::BlockHeader(header) => header.to_string(),
            RxMsgs::TxId(id) => format!("txid: {}", id.to_owned()),
            RxMsgs::SubscriptionResult(_subscription_result) => "sub success".to_owned(),
        }
    }
}

#[derive(serde::Deserialize)]
pub(crate) struct NewHeader {
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
pub(crate) struct SubscriptionResult {
    pub(crate) id: String,
    pub(crate) result: String,
}

#[derive(serde::Deserialize)]
pub(crate) struct UnconfirmedTx {
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
