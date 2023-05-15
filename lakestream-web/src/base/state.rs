use web_sys::CryptoKey;

#[derive(Default, Clone, Debug)]
pub struct GlobalState {
    pub crypto_key: Option<CryptoKey>,
    pub old_key: Option<String>,
    pub counter: u32,
}
