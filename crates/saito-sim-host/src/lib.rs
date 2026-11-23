//! Cross-platform host runtime and local store for sandboxed historical WASM simulation kernels
//! (`saito-sim-<line>.wasm`, S-022).
//!
//! - Desktop & Android: Wasmtime with Cranelift JIT and standard Wasm Exception Handling (`try_table`).
//! - iOS (or `SAITO_WASM_PULLEY=1`): Wasmtime with the portable `pulley64` bytecode interpreter (no JIT).
//! - Sandbox: read-only WASI preopened directories for the replay's content bundle, background pool,
//!   and ROMs — zero write, network, or environment access.

#![forbid(unsafe_code)]

pub mod runtime;
pub mod store;

pub use runtime::{WasmRuntime, WasmViewer};
pub use saito_sim_abi::{
    ABI_VERSION, DEFAULT_KERNELS_INDEX, KERNELS_DIR_ENV, KERNELS_INDEX_ENV, KernelIndex, KernelRecord,
    PartyMemberSnapshot, PartySnapshot, PublishedKernel,
};
pub use store::{KernelStore, StoredKernel, sha256_hex};
