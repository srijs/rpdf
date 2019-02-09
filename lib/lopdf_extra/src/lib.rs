use failure::Fallible;

mod de;

pub trait DocumentExt {
    fn resolve_object<'a>(&'a self, obj: &'a lopdf::Object) -> Fallible<&'a lopdf::Object>;

    fn deserialize_object<'de, T>(&'de self, obj: &'de lopdf::Object) -> Fallible<T>
    where
        T: serde::Deserialize<'de>;
}

impl DocumentExt for lopdf::Document {
    fn resolve_object<'a>(&'a self, obj: &'a lopdf::Object) -> Fallible<&'a lopdf::Object> {
        if let lopdf::Object::Reference(oid) = obj {
            self.get_object(*oid)
                .ok_or_else(|| failure::format_err!("object {:?} not found", oid))
        } else {
            Ok(obj)
        }
    }

    fn deserialize_object<'de, T>(&'de self, obj: &'de lopdf::Object) -> Fallible<T>
    where
        T: serde::Deserialize<'de>,
    {
        Ok(T::deserialize(de::ObjectDeserializer::new(self, obj))?)
    }
}

pub trait DictionaryExt {
    fn try_get(&self, key: &[u8]) -> Fallible<&lopdf::Object>;
}

impl DictionaryExt for lopdf::Dictionary {
    fn try_get(&self, key: &[u8]) -> Fallible<&lopdf::Object> {
        self.get(key)
            .ok_or_else(|| failure::format_err!("key {:?} not found", key))
    }
}

pub trait ObjectExt {
    fn try_as_stream(&self) -> Fallible<&lopdf::Stream>;

    fn try_as_dict(&self) -> Fallible<&lopdf::Dictionary>;
}

impl ObjectExt for lopdf::Object {
    fn try_as_stream(&self) -> Fallible<&lopdf::Stream> {
        self.as_stream()
            .ok_or_else(|| failure::format_err!("object is not a stream"))
    }

    fn try_as_dict(&self) -> Fallible<&lopdf::Dictionary> {
        self.as_dict()
            .ok_or_else(|| failure::format_err!("object is not a dictionary"))
    }
}
