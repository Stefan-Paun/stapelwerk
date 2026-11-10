//! WebAssembly ABI v1 of Stapelwerk (`docs/ABI.md`).
//!
//! Built as a `cdylib` for `wasm32-unknown-unknown` with **no imports**. Exports use only
//! `i32`/`u32`. This crate is the only place in the workspace where `unsafe` is allowed (the
//! `#[unsafe(no_mangle)]` attribute and, later, raw buffer access at the JS boundary).
//!
//! Phase 0 exports only [`abi_version`]; the full export list is `docs/abi-exports.txt`.

// Keeps the dependency edge to the core in place for the later phases.
use stapelwerk_core as _;

/// ABI version. JavaScript (`web/js/vm.js`) refuses to run with any other value.
#[unsafe(no_mangle)]
pub extern "C" fn abi_version() -> u32 {
    1
}

#[cfg(test)]
mod tests {
    #[test]
    fn version_is_one() {
        assert_eq!(super::abi_version(), 1);
    }
}
