use std::sync::Arc;

use failure::Fallible;

pub struct LoadedFont {
    inner: font_kit::loaders::default::Font,
}

impl LoadedFont {
    pub fn from_bytes(bytes: Arc<Vec<u8>>) -> Fallible<Self> {
        Ok(LoadedFont {
            inner: font_kit::loaders::default::Font::from_bytes(bytes, 0)?,
        })
    }

    pub fn glyph_index_for_name(&self, name: &[u8]) -> u32 {
        if let Ok(name_str) = std::str::from_utf8(name) {
            self.inner.glyph_by_name(name_str).unwrap_or(0)
        } else {
            0
        }
    }

    pub fn glyph_index_for_char(&self, character: char) -> u32 {
        self.inner.glyph_for_char(character).unwrap_or(0)
    }
}
