//! Native desktop entry point. The web build uses the `#[wasm_bindgen(start)]`
//! shim in `lib.rs` instead, so this file is native-only.

fn main() {
    scraped_again::run();
}
