use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use failure::Fallible;

use crate::pdf::data::{Name, Number, TryFromObject};

pub struct Font {
    first_char: i64,
    last_char: i64,
    widths: Vec<Number>,
    data: Option<FontData>,
}

pub enum FontData {
    Type1(Arc<Vec<u8>>),
}

impl Font {
    pub fn try_from_dictionary(doc: &lopdf::Document, dict: &lopdf::Dictionary) -> Fallible<Self> {
        let Name(subtype) = Name::try_from_object(doc, dict.get(b"Subtype").unwrap())?;

        let descriptor =
            <&lopdf::Dictionary>::try_from_object(doc, dict.get(b"FontDescriptor").unwrap())?;

        let data = match subtype.as_slice() {
            b"Type1" => {
                let file =
                    <&lopdf::Stream>::try_from_object(doc, descriptor.get(b"FontFile").unwrap())?;
                if let Some(content) = file.decompressed_content() {
                    Some(FontData::Type1(Arc::new(content)))
                } else {
                    Some(FontData::Type1(Arc::new(file.content.clone())))
                }
            }
            _ => None,
        };

        if let Some(encoding) = dict.get(b"Encoding") {
            //log::debug!("Font has encoding {:?}", encoding);
        }

        let first_char = i64::try_from_object(doc, dict.get(b"FirstChar").unwrap())?;
        let last_char = i64::try_from_object(doc, dict.get(b"LastChar").unwrap())?;
        let widths = Vec::<Number>::try_from_object(doc, dict.get(b"Widths").unwrap())?;

        Ok(Font {
            first_char,
            last_char,
            widths,
            data,
        })
    }

    pub fn data(&self) -> Option<&FontData> {
        self.data.as_ref()
    }

    pub fn char_code(&self, c: u8) -> Option<u32> {
        if i64::from(c) < self.first_char || i64::from(c) > self.last_char {
            return None;
        }
        let index = i64::from(c) - self.first_char;
        Some(index as u32)
    }

    pub fn width_for_char(&self, c: u8) -> f64 {
        if i64::from(c) < self.first_char || i64::from(c) > self.last_char {
            return 0.0;
        }
        let index = i64::from(c) - self.first_char;
        let Number(width) = self.widths[index as usize];
        width / 1000.0
    }
}

pub struct FontMap {
    map: HashMap<Vec<u8>, Font>,
}

impl FontMap {
    pub fn try_from_page_fonts(
        doc: &lopdf::Document,
        page_fonts: BTreeMap<Vec<u8>, &lopdf::Dictionary>,
    ) -> Fallible<Self> {
        let map = page_fonts
            .into_iter()
            .map(|(name, dict)| Ok((name, Font::try_from_dictionary(doc, dict)?)))
            .collect::<Fallible<HashMap<_, _>>>()?;
        Ok(FontMap { map })
    }

    pub fn iter(&self) -> impl Iterator<Item = (&[u8], &Font)> {
        self.map.iter().map(|(name, font)| (name.as_slice(), font))
    }

    pub fn get(&self, name: &[u8]) -> Option<&Font> {
        self.map.get(name)
    }
}
