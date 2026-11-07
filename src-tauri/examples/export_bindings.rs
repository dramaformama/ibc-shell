//! `just bindings`: writes `ui/src/lib/api/bindings.ts` from the tauri-specta builder.

fn main() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../ui/src/lib/api/bindings.ts");
    if let Err(err) = termlite_lib::api::export_bindings(&path) {
        eprintln!("bindings export failed: {err}");
        std::process::exit(1);
    }
    println!("wrote {}", path.display());
}
