use std::collections::BTreeMap;

use failure::Fallible;

use crate::pdf::data::{Name, TryFromObject};

#[derive(Debug)]
pub enum Encoding {
    Predefined(PredefinedEncoding),
    Dictionary(EncodingDictionary),
}

impl Encoding {
    pub fn lookup(&self, char_code: u8) -> Option<&GlyphName> {
        match self {
            Encoding::Predefined(_) => {
                // TODO: consult predefined lookup table
                None
            }
            Encoding::Dictionary(enc) => {
                enc.lookup(char_code)
            }
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
    pub differences: BTreeMap<u8, GlyphName>,
}

impl EncodingDictionary {
    pub fn lookup(&self, char_code: u8) -> Option<&GlyphName> {
        if let Some(glyph_name) = self.differences.get(&char_code) {
            Some(glyph_name)
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
                            differences.insert(index as u8, GlyphName(name.clone()));
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

#[derive(Clone, Debug)]
pub struct GlyphName(Vec<u8>);

impl GlyphName {
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_slice()
    }

    pub fn to_char(&self) -> char {
        rpdf_glyph_names::glyph_name_to_char(&self.0)
    }
}
