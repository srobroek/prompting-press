fn main() {
    // PyO3's `extension-module` feature leaves CPython symbols (`__Py_NoneStruct`,
    // `_Py_IncRef`, ...) undefined for the interpreter to resolve at load time. A bare
    // `cargo build` of this standalone cdylib has no interpreter to link against, so on
    // macOS the linker (unlike Linux ELF, which permits undefined symbols in a dylib)
    // needs `-undefined dynamic_lookup` to defer their resolution to load time.
    //
    // Scoped here via `cargo:rustc-link-arg` (not a repo-wide .cargo/config.toml) so the
    // flags attach ONLY to this crate's link phase and never enter the RUSTFLAGS
    // fingerprint — keeping the US3 codegen-determinism gate and the US4 `cargo tree`
    // FFI-isolation gate unperturbed. Linux/Windows need nothing.
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-arg=-undefined");
        println!("cargo:rustc-link-arg=dynamic_lookup");
    }
}
