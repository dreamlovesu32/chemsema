use chemsema_engine::cdx_to_cdxml;
use std::{env, fs};

fn main() {
    let mut args = env::args().skip(1);
    let input = args.next().expect(
        "usage: cargo run -p chemsema-engine --example cdx_to_cdxml -- <input.cdx> [output.cdxml]",
    );
    let output = args.next();
    let bytes = fs::read(&input).expect("CDX input should be readable");
    let cdxml = cdx_to_cdxml(&bytes).expect("CDX should convert to CDXML");
    if let Some(output) = output {
        fs::write(output, cdxml).expect("CDXML output should be writable");
    } else {
        print!("{cdxml}");
    }
}
