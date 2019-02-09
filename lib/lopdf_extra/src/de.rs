use std::fmt;

use serde::de::Visitor;

#[derive(Debug)]
pub struct Error(String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for Error {}

impl serde::de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error(msg.to_string())
    }
}

struct SeqAccess<'a> {
    document: &'a lopdf::Document,
    objects: &'a [lopdf::Object],
}

impl<'de> serde::de::SeqAccess<'de> for SeqAccess<'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        if let Some((object, objects)) = self.objects.split_first() {
            self.objects = objects;
            let value = serde::de::DeserializeSeed::deserialize(
                seed,
                ObjectDeserializer {
                    document: self.document,
                    object,
                },
            )?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.objects.len())
    }
}

struct MapAccess<'a, I> {
    document: &'a lopdf::Document,
    iterator: I,
    object: Option<&'a lopdf::Object>,
}

impl<'de, I> serde::de::MapAccess<'de> for MapAccess<'de, I>
where
    I: Iterator<Item = (&'de Vec<u8>, &'de lopdf::Object)> + ExactSizeIterator + 'de,
{
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        if let Some((key, object)) = self.iterator.next() {
            self.object = Some(object);
            let value = serde::de::DeserializeSeed::deserialize(
                seed,
                serde::de::value::BorrowedBytesDeserializer::new(key.as_slice()),
            )?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        serde::de::DeserializeSeed::deserialize(
            seed,
            ObjectDeserializer {
                document: self.document,
                object: self
                    .object
                    .take()
                    .ok_or_else(|| Error("internal error".to_owned()))?,
            },
        )
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.iterator.len())
    }
}

pub struct ObjectDeserializer<'a> {
    document: &'a lopdf::Document,
    object: &'a lopdf::Object,
}

impl<'a> ObjectDeserializer<'a> {
    pub fn new(document: &'a lopdf::Document, object: &'a lopdf::Object) -> Self {
        ObjectDeserializer { document, object }
    }
}

impl<'de> serde::Deserializer<'de> for ObjectDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.object {
            lopdf::Object::Null => visitor.visit_none(),
            lopdf::Object::Boolean(v) => visitor.visit_bool(*v),
            lopdf::Object::Integer(v) => visitor.visit_i64(*v),
            lopdf::Object::Real(v) => visitor.visit_f64(*v),
            lopdf::Object::Name(v) => visitor.visit_borrowed_bytes(v.as_slice()),
            lopdf::Object::String(v, _) => visitor.visit_borrowed_bytes(v.as_slice()),
            lopdf::Object::Array(v) => visitor.visit_seq(SeqAccess {
                document: self.document,
                objects: v.as_slice(),
            }),
            lopdf::Object::Dictionary(v) => visitor.visit_map(MapAccess {
                document: self.document,
                iterator: v.iter(),
                object: None,
            }),
            lopdf::Object::Stream(stream) => visitor.visit_map(MapAccess {
                document: self.document,
                iterator: stream.dict.iter(),
                object: None,
            }),
            lopdf::Object::Reference(oid) => {
                if let Some(object) = self.document.get_object(*oid) {
                    let deserializer = ObjectDeserializer {
                        document: self.document,
                        object,
                    };
                    deserializer.deserialize_any(visitor)
                } else {
                    Err(Error(format!("object with id {:?} not found", oid)))
                }
            }
        }
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}
