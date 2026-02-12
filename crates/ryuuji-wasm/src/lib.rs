use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn parse_filename(input: &str) -> String {
    let elements = ryuuji_parse::parse(input);
    serde_json::to_string(&elements).unwrap_or_else(|_| "{}".to_string())
}
