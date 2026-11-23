// The targets by name: `#[cfg(web)]` (a browser) and natively the rest.
fn main() {
    cfg_aliases::cfg_aliases! {
        web: { all(target_arch = "wasm32", target_os = "unknown") },
    }
}
