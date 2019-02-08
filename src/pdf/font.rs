use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use failure::Fallible;

use crate::pdf::data::{Name, Number, TryFromObject};

mod encoding;
use self::encoding::Encoding;
pub use self::encoding::GlyphName;
mod loaded;
pub use self::loaded::LoadedFont;

#[derive(Clone, Copy, Debug)]
pub enum Subtype {
    Type1,
}

pub struct Font {
    first_char: i64,
    last_char: i64,
    widths: Vec<Number>,
    data: Arc<Vec<u8>>,
    subtype: Subtype,
    encoding: Option<encoding::Encoding>,
}

impl Font {
    pub fn try_from_dictionary(doc: &lopdf::Document, dict: &lopdf::Dictionary) -> Fallible<Self> {
        let Name(subtype_name) = Name::try_from_object(doc, dict.get(b"Subtype").unwrap())?;
        let subtype = match subtype_name.as_slice() {
            b"Type1" => Subtype::Type1,
            _ => failure::bail!("unsupported font subtype"),
        };

        let mut encoding = None;
        if let Some(encoding_obj) = dict.get(b"Encoding") {
            encoding = Some(Encoding::try_from_object(doc, encoding_obj)?);
            log::debug!("font has encoding {:?}", encoding);
        }

        let data;
        match subtype {
            Subtype::Type1 => {
                let descriptor = <&lopdf::Dictionary>::try_from_object(
                    doc,
                    dict.get(b"FontDescriptor").unwrap(),
                )?;

                if let Some(file_obj) = descriptor.get(b"FontFile") {
                    let file = <&lopdf::Stream>::try_from_object(doc, file_obj)?;
                    if let Some(content) = file.decompressed_content() {
                        data = Arc::new(content);
                    } else {
                        data = Arc::new(file.content.clone());
                    }
                } else {
                    failure::bail!("font is missing glyph data");
                }
            }
        };

        let first_char = i64::try_from_object(doc, dict.get(b"FirstChar").unwrap())?;
        let last_char = i64::try_from_object(doc, dict.get(b"LastChar").unwrap())?;
        let widths = Vec::<Number>::try_from_object(doc, dict.get(b"Widths").unwrap())?;

        Ok(Font {
            first_char,
            last_char,
            widths,
            data,
            subtype,
            encoding,
        })
    }

    pub fn data(&self) -> &[u8] {
        self.data.as_slice()
    }

    pub fn decode_char(&self, c: u8) -> Option<&GlyphName> {
        if let Some(ref encoding) = self.encoding {
            encoding.lookup(c)
        } else {
            None
        }
    }

    pub fn width_for_char(&self, c: u8) -> f64 {
        if i64::from(c) < self.first_char || i64::from(c) > self.last_char {
            return 0.0;
        }
        let index = i64::from(c) - self.first_char;
        let Number(width) = self.widths[index as usize];
        width / 1000.0
    }

    pub fn load(&self) -> Fallible<LoadedFont> {
        LoadedFont::from_bytes(self.data.clone())
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
            .flat_map(|(name, dict)| Some((name, Font::try_from_dictionary(doc, dict).ok()?)))
            .collect::<HashMap<_, _>>();
        Ok(FontMap { map })
    }

    pub fn get(&self, name: &[u8]) -> Option<&Font> {
        self.map.get(name)
    }
}
