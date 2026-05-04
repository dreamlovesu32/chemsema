use chemcore_engine::{document_to_svg, parse_cdxml_document};

fn main() {
    let mut args = std::env::args().skip(1);
    let input = args
        .next()
        .unwrap_or_else(|| "tmp/arrows.cdxml".to_string());
    let output = args
        .next()
        .unwrap_or_else(|| "tmp/arrows-backend.svg".to_string());

    let cdxml = std::fs::read_to_string(&input).expect("cdxml should be readable");
    let document = parse_cdxml_document(&cdxml, Some(&input)).expect("cdxml should parse");
    let svg = document_to_svg(&document);
    std::fs::write(&output, svg).expect("svg should be writable");
    println!("wrote {output}");
}
