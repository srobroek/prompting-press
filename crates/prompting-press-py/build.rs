use std::process::Command;

fn main() {
    // Rebuild this script if the interpreter selection changes.
    println!("cargo:rerun-if-env-changed=PYO3_PYTHON");
    println!("cargo:rerun-if-changed=build.rs");

    // Two distinct link modes, keyed on the `extension-module` feature:
    //
    // 1. `extension-module` ON (the maturin wheel build): PyO3 leaves CPython symbols
    //    (`__Py_NoneStruct`, `_Py_IncRef`, ...) undefined for the interpreter to resolve at
    //    load time and does NOT link libpython. A bare link of this standalone cdylib then has
    //    no interpreter to satisfy those symbols, so on macOS (unlike Linux ELF, which permits
    //    undefined symbols in a dylib) the linker needs `-undefined dynamic_lookup` to defer
    //    resolution to load time.
    //
    // 2. `extension-module` OFF (the dev / `cargo test` build that T012/T025 run): PyO3 links
    //    libpython so the `#[cfg(test)]` `Python::attach` unit tests can execute. On macOS the
    //    resulting test binary then needs an `-rpath` to the interpreter's LIBDIR so
    //    `libpythonX.Y.dylib` is found at runtime (otherwise: `Library not loaded:
    //    @rpath/libpython3.x.dylib`).
    //
    // Both arms are scoped via `cargo:rustc-link-arg` (not a repo-wide .cargo/config.toml) so
    // the flags attach ONLY to this crate's link phase and never enter the RUSTFLAGS
    // fingerprint — keeping the codegen-determinism gate and the `cargo tree` FFI-isolation
    // gate unperturbed.
    let extension_module = std::env::var_os("CARGO_FEATURE_EXTENSION_MODULE").is_some();

    // Windows resolves the interpreter via an import library (PyO3's own build script handles
    // it); no link-arg is needed here, and the dev `cargo test` binary finds the DLL via PATH.
    if cfg!(target_os = "windows") {
        return;
    }

    if extension_module {
        // Wheel build: defer CPython symbol resolution to load time. Only macOS needs the flag —
        // Linux ELF already permits undefined symbols in a shared object (resolved at load).
        if cfg!(target_os = "macos") {
            println!("cargo:rustc-link-arg=-undefined");
            println!("cargo:rustc-link-arg=dynamic_lookup");
        }
    } else if let Some(libdir) = python_libdir() {
        // Dev / test build (`extension-module` OFF): PyO3 links libpython, so the resulting
        // `cargo test` binary must find it at runtime. Embed an rpath to the interpreter's
        // LIBDIR. Needed on BOTH macOS (dyld) AND Linux (ld.so) when the interpreter lives
        // outside the system loader path — e.g. a mise-/pyenv-managed Python, where the bare
        // binary otherwise fails with `libpython3.x.so: cannot open shared object file`.
        // `-Wl,-rpath,{dir}` is honored by both linkers (DT_RUNPATH covers the direct libpython
        // dependency on Linux).
        println!("cargo:rustc-link-arg=-Wl,-rpath,{libdir}");
    }
}

/// Ask the interpreter PyO3 will use for its library directory.
///
/// Mirrors PyO3's own interpreter selection: honor `PYO3_PYTHON`, else fall back to `python3`
/// / `python` on `PATH`. Returns `None` if no interpreter can be queried (in which case the
/// dev build simply omits the rpath — the wheel build does not need it).
fn python_libdir() -> Option<String> {
    let interpreters = match std::env::var("PYO3_PYTHON") {
        Ok(p) if !p.is_empty() => vec![p],
        _ => vec!["python3".to_string(), "python".to_string()],
    };

    for interp in interpreters {
        let output = Command::new(&interp)
            .args([
                "-c",
                "import sysconfig; print(sysconfig.get_config_var('LIBDIR') or '')",
            ])
            .output();
        if let Ok(out) = output {
            if out.status.success() {
                let libdir = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !libdir.is_empty() {
                    return Some(libdir);
                }
            }
        }
    }
    None
}
