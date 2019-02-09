use serde_derive::Deserialize;

mod differences;

#[derive(Debug, Deserialize, PartialEq)]
#[serde(untagged)]
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
            Encoding::Dictionary(enc) => enc.lookup(char_code),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(variant_identifier)]
pub enum PredefinedEncoding {
    MacRomanEncoding,
    MacExpertEncoding,
    WinAnsiEncoding,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct EncodingDictionary {
    #[serde(rename = "BaseEncoding")]
    base: Option<PredefinedEncoding>,
    #[serde(rename = "Differences", default)]
    differences: differences::Differences,
}

impl EncodingDictionary {
    pub fn lookup(&self, char_code: u8) -> Option<&GlyphName> {
        if let Some(glyph_name) = self.differences.lookup(char_code) {
            Some(glyph_name)
        } else {
            // TODO: consult base lookup table
            None
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GlyphName(Vec<u8>);

impl GlyphName {
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_slice()
    }

    pub fn to_char(&self) -> char {
        rpdf_glyph_names::glyph_name_to_char(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rpdf_lopdf_extra::DocumentExt;

    #[test]
    fn deserialize_encoding_predefined() {
        assert_eq!(
            Encoding::Predefined(PredefinedEncoding::MacRomanEncoding),
            lopdf::Document::new()
                .deserialize_object(&lopdf::Object::from("MacRomanEncoding"))
                .unwrap()
        );
        assert_eq!(
            Encoding::Predefined(PredefinedEncoding::MacExpertEncoding),
            lopdf::Document::new()
                .deserialize_object(&lopdf::Object::from("MacExpertEncoding"))
                .unwrap()
        );
        assert_eq!(
            Encoding::Predefined(PredefinedEncoding::WinAnsiEncoding),
            lopdf::Document::new()
                .deserialize_object(&lopdf::Object::from("WinAnsiEncoding"))
                .unwrap()
        );
    }
}
