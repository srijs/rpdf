use std::collections::BTreeMap;
use std::fmt;

use super::GlyphName;

#[derive(Debug, Default, PartialEq)]
pub struct Differences(BTreeMap<u8, GlyphName>);

impl Differences {
    pub fn lookup(&self, char_code: u8) -> Option<&GlyphName> {
        self.0.get(&char_code)
    }
}

impl<'de> serde::Deserialize<'de> for Differences {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(SeqVisitor)
    }
}

struct SeqVisitor;

impl<'de> serde::de::Visitor<'de> for SeqVisitor {
    type Value = Differences;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "an array of differences")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut index = 0u32;
        let mut map = BTreeMap::new();
        loop {
            let seed = ElementVisitor {
                index: &mut index,
                map: &mut map,
            };
            if serde::de::SeqAccess::next_element_seed(&mut seq, seed)?.is_none() {
                return Ok(Differences(map));
            }
        }
    }
}

struct ElementVisitor<'a> {
    index: &'a mut u32,
    map: &'a mut BTreeMap<u8, GlyphName>,
}

impl<'de, 'a> serde::de::DeserializeSeed<'de> for ElementVisitor<'a> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }
}

impl<'a, 'de> serde::de::Visitor<'de> for ElementVisitor<'a> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a name or an integer")
    }

    fn visit_borrowed_bytes<E>(self, bytes: &'de [u8]) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.map
            .insert(*self.index as u8, GlyphName(bytes.to_owned()));
        *self.index += 1;
        Ok(())
    }

    fn visit_i64<E>(self, i: i64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        *self.index = i as u32;
        Ok(())
    }
}
