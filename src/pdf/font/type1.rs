use std::collections::BTreeMap;

use failure::Fallible;

use crate::pdf::data::{Name, TryFromObject};

const UNICODE_REPLACEMENT_CHAR: char = '\u{FFFD}';

#[derive(Debug)]
pub enum Encoding {
    Predefined(PredefinedEncoding),
    Dictionary(EncodingDictionary),
}

impl Encoding {
    pub fn translate(&self, char_code: u8) -> char {
        match self {
            Encoding::Predefined(_) => {
                // TODO: consult predefined lookup table
                char_code as char
            }
            Encoding::Dictionary(enc) => enc.lookup(char_code as u32).unwrap_or(char_code as char),
        }
    }
}

impl<'a> TryFromObject<'a> for Encoding {
    fn try_from_object_direct(doc: &'a lopdf::Document, obj: &'a lopdf::Object) -> Fallible<Self>
    where
        Self: Sized,
    {
        match obj {
            lopdf::Object::Name(_) => Ok(Encoding::Predefined(
                PredefinedEncoding::try_from_object_direct(doc, obj)?,
            )),
            lopdf::Object::Dictionary(_) => Ok(Encoding::Dictionary(
                EncodingDictionary::try_from_object_direct(doc, obj)?,
            )),
            _ => failure::bail!("unexpected object type"),
        }
    }
}

#[derive(Debug)]
pub enum PredefinedEncoding {
    MacRomanEncoding,
    MacExpertEncoding,
    WinAnsiEncoding,
}

impl<'a> TryFromObject<'a> for PredefinedEncoding {
    fn try_from_object_direct(doc: &'a lopdf::Document, obj: &'a lopdf::Object) -> Fallible<Self>
    where
        Self: Sized,
    {
        let Name(name) = Name::try_from_object(doc, obj)?;
        match name.as_slice() {
            b"MacRomanEncoding" => Ok(PredefinedEncoding::MacRomanEncoding),
            b"MacExpertEncoding" => Ok(PredefinedEncoding::MacExpertEncoding),
            b"WinAnsiEncoding" => Ok(PredefinedEncoding::WinAnsiEncoding),
            _ => failure::bail!("unknown predefined encoding"),
        }
    }
}

#[derive(Debug)]
pub struct EncodingDictionary {
    pub base: Option<PredefinedEncoding>,
    pub differences: BTreeMap<u32, char>,
}

impl EncodingDictionary {
    pub fn lookup(&self, char_code: u32) -> Option<char> {
        if let Some(c) = self.differences.get(&char_code) {
            Some(*c)
        } else {
            // TODO: consult base lookup table
            None
        }
    }
}

impl<'a> TryFromObject<'a> for EncodingDictionary {
    fn try_from_object_direct(doc: &'a lopdf::Document, obj: &'a lopdf::Object) -> Fallible<Self>
    where
        Self: Sized,
    {
        let dict = <&lopdf::Dictionary>::try_from_object(doc, obj)?;

        let mut base = None;
        if let Some(base_obj) = dict.get(b"BaseEncoding") {
            base = Some(PredefinedEncoding::try_from_object(doc, base_obj)?);
        }

        let mut differences = BTreeMap::new();
        if let Some(diff_obj) = dict.get(b"Differences") {
            let diff_obj_direct = <&lopdf::Object>::try_from_object(doc, diff_obj)?;

            if let Some(elements) = diff_obj_direct.as_array() {
                let mut index = 0u32;
                for element in elements {
                    match element {
                        lopdf::Object::Integer(code) => {
                            index = *code as u32;
                        }
                        lopdf::Object::Name(name) => {
                            differences.insert(index, rpdf_glyph_names::glyph_name_to_char(name));
                            index += 1;
                        }
                        _ => {}
                    }
                }
            } else {
                failure::bail!("difference object is not an array");
            }
        }

        Ok(EncodingDictionary { base, differences })
    }
}
