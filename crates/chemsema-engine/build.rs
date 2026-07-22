use flate2::{write::GzEncoder, Compression};
use std::{env, fs, io::Write, path::PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let source = manifest_dir.join("../../shared/glyph_outlines.json");
    println!("cargo:rerun-if-changed={}", source.display());

    let bytes = fs::read(&source).expect("read glyph outline manifest");
    let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(&bytes).expect("compress glyph outlines");
    let compressed = encoder.finish().expect("finish glyph compression");

    let output =
        PathBuf::from(env::var_os("OUT_DIR").expect("out dir")).join("glyph_outlines.json.gz");
    fs::write(output, compressed).expect("write compressed glyph outlines");
}
