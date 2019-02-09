pub mod de;

pub trait DocumentExt {
    fn resolve_object<'a>(&'a self, obj: &'a lopdf::Object) -> &'a lopdf::Object;

    fn deserialize_object<'de, T>(&'de self, obj: &'de lopdf::Object) -> Result<T, de::Error>
    where
        T: serde::Deserialize<'de>;
}

impl DocumentExt for lopdf::Document {
    fn resolve_object<'a>(&'a self, obj: &'a lopdf::Object) -> &'a lopdf::Object {
        if let lopdf::Object::Reference(oid) = obj {
            self.get_object(*oid).unwrap()
        } else {
            obj
        }
    }

    fn deserialize_object<'de, T>(&'de self, obj: &'de lopdf::Object) -> Result<T, de::Error>
    where
        T: serde::Deserialize<'de>,
    {
        T::deserialize(de::ObjectDeserializer::new(self, obj))
    }
}
