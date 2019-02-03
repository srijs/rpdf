const UNICODE_REPLACEMENT_CHAR: char = '\u{FFFD}';

include!(concat!(env!("OUT_DIR"), "/codegen.rs"));

pub fn glyph_name_to_char(name: &[u8]) -> char {
    GLYPH_MAP
        .get(name)
        .cloned()
        .unwrap_or(UNICODE_REPLACEMENT_CHAR)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glyph_map() {
        assert_eq!('\u{24B6}', glyph_name_to_char(b"Acircle"));
    }
}
