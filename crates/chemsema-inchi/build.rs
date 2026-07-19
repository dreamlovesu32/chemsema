use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let target = env::var("TARGET").unwrap_or_default();
    if target.starts_with("wasm32") {
        return;
    }
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("../../third_party/inchi-1.07.5");
    let base = root.join("INCHI-1-SRC/INCHI_BASE/src");
    let api = root.join("INCHI-1-SRC/INCHI_API/libinchi/src");
    let mut sources = Vec::new();
    collect_c_sources(&base, &mut sources);
    collect_c_sources(&api, &mut sources);
    sources.sort();

    let mut build = cc::Build::new();
    build
        .files(&sources)
        .include(&base)
        .include(&api)
        .include(api.join("ixa"))
        .define("COMPILE_ANSI_ONLY", None)
        .define("TARGET_API_LIB", None)
        .define("_CRT_SECURE_NO_WARNINGS", None)
        .warnings(false);
    if !target.contains("msvc") {
        build.flag_if_supported("-fno-strict-aliasing");
    }
    build.compile("inchi");

    println!("cargo:rerun-if-changed={}", base.display());
    println!("cargo:rerun-if-changed={}", api.display());
}

fn collect_c_sources(path: &Path, output: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(path) else {
        panic!(
            "missing vendored InChI source directory: {}",
            path.display()
        );
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_c_sources(&path, output);
        } else if path.extension().and_then(|value| value.to_str()) == Some("c") {
            output.push(path);
        }
    }
}
