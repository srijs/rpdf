use std::fmt;

pub struct Rectangle {
    lower_left_x: f64,
    lower_left_y: f64,
    upper_right_x: f64,
    upper_right_y: f64,
}

impl Rectangle {
    pub fn width(&self) -> f64 {
        (self.upper_right_x - self.lower_left_x).abs()
    }

    pub fn height(&self) -> f64 {
        (self.upper_right_y - self.lower_left_y).abs()
    }
}

impl<'de> serde::Deserialize<'de> for Rectangle {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (lower_left_x, lower_left_y, upper_right_x, upper_right_y) =
            serde::Deserialize::deserialize(deserializer)?;
        Ok(Rectangle {
            lower_left_x,
            lower_left_y,
            upper_right_x,
            upper_right_y,
        })
    }
}

pub struct Name(pub Vec<u8>);

impl<'de> serde::Deserialize<'de> for Name {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Name;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "a byte sequence")
            }

            fn visit_borrowed_bytes<E>(self, bytes: &'de [u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Name(bytes.to_owned()))
            }
        }

        deserializer.deserialize_bytes(Visitor)
    }
}
