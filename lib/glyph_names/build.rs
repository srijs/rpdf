use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

static DATA_FILE: &str = "data/glyphlist-extended.txt";

fn main() {
    let mut map = HashMap::new();

    let lines = BufReader::new(File::open(DATA_FILE).unwrap()).lines();
    for line_result in lines {
        let line = line_result.unwrap();
        if line.starts_with("#") || line.is_empty() {
            continue;
        }

        let parts = line.split(|c| c == ';' || c == ' ').collect::<Vec<_>>();

        let c32 = u32::from_str_radix(parts[1], 16).unwrap();
        if let Some(c) = std::char::from_u32(c32) {
            map.insert(parts[0].to_owned(), format!("'\\u{{{:x}}}'", c as u32));
        }
    }

    let path = Path::new(&env::var("OUT_DIR").unwrap()).join("codegen.rs");
    let mut file = BufWriter::new(File::create(&path).unwrap());
    write!(
        &mut file,
        "static GLYPH_MAP: phf::Map<&'static str, char> = "
    )
    .unwrap();
    let mut map_builder = phf_codegen::Map::new();
    for (key, value) in map {
        map_builder.entry(key, &value);
    }
    map_builder.build(&mut file).unwrap();
    write!(&mut file, ";\n").unwrap();
}
