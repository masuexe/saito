// The targets by name: `#[cfg(web)]` (a browser), `#[cfg(ios)]`, `#[cfg(android)]`, `#[cfg(desktop)]` (the rest).
fn main() {
    cfg_aliases::cfg_aliases! {
        web: { all(target_arch = "wasm32", target_os = "unknown") },
        ios: { target_os = "ios" },
        android: { target_os = "android" },
        desktop: { not(any(web, ios, android)) },
    }
}
