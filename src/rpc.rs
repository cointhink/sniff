use serde::Serialize;

#[derive(Serialize)]
pub struct RpcCall<'a> {
    jsonrpc: &'static str,
    id: String,
    method: &'a str,
    params: Vec<serde_json::Value>,
}

impl<'a> RpcCall<'a> {
    pub fn new(method: &'a str, params: Vec<serde_json::Value>) -> Self {
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
