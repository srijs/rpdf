use failure::{bail, format_err, Fallible};

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

pub struct Number(pub f64);

pub trait TryFromObject<'a> {
    fn try_from_object_direct(doc: &'a lopdf::Document, obj: &'a lopdf::Object) -> Fallible<Self>
    where
        Self: Sized;

    fn try_from_object(doc: &'a lopdf::Document, obj: &'a lopdf::Object) -> Fallible<Self>
    where
        Self: Sized,
    {
        match obj {
            lopdf::Object::Reference(id) => {
                if let Some(inner_obj) = doc.get_object(*id) {
                    Self::try_from_object(doc, inner_obj)
                } else {
                    bail!("reference target not found")
                }
            }
            _ => Self::try_from_object_direct(doc, obj),
        }
    }
}

impl<'a> TryFromObject<'a> for Number {
    fn try_from_object_direct(_doc: &'a lopdf::Document, obj: &'a lopdf::Object) -> Fallible<Self>
    where
        Self: Sized,
    {
        match obj {
            lopdf::Object::Integer(i) => Ok(Number(*i as f64)),
            lopdf::Object::Real(r) => Ok(Number(*r)),
            _ => bail!("unexpected object type"),
        }
    }
}

impl<'a> TryFromObject<'a> for i64 {
    fn try_from_object_direct(_doc: &'a lopdf::Document, obj: &'a lopdf::Object) -> Fallible<Self>
    where
        Self: Sized,
    {
        obj.as_i64()
            .ok_or_else(|| format_err!("unexpected object type"))
    }
}

pub struct Name(pub Vec<u8>);

impl<'a> TryFromObject<'a> for Name {
    fn try_from_object_direct(_doc: &'a lopdf::Document, obj: &'a lopdf::Object) -> Fallible<Self>
    where
        Self: Sized,
    {
        match obj {
            lopdf::Object::Name(data) => Ok(Name(data.clone())),
            _ => bail!("unexpected object type"),
        }
    }
}

impl<'a> TryFromObject<'a> for &'a lopdf::Stream {
    fn try_from_object_direct(_doc: &'a lopdf::Document, obj: &'a lopdf::Object) -> Fallible<Self>
    where
        Self: Sized,
    {
        match obj {
            lopdf::Object::Stream(stream) => Ok(stream),
            _ => bail!("unexpected object type"),
        }
    }
}

impl<'a> TryFromObject<'a> for &'a lopdf::Dictionary {
    fn try_from_object_direct(_doc: &'a lopdf::Document, obj: &'a lopdf::Object) -> Fallible<Self>
    where
        Self: Sized,
    {
        match obj {
            lopdf::Object::Dictionary(dict) => Ok(dict),
            _ => bail!("unexpected object type"),
        }
    }
}

impl<'a, T> TryFromObject<'a> for Vec<T>
where
    T: TryFromObject<'a>,
{
    fn try_from_object_direct(doc: &'a lopdf::Document, obj: &'a lopdf::Object) -> Fallible<Self>
    where
        Self: Sized,
    {
        match obj {
            lopdf::Object::Array(objs) => {
                let mut array = Vec::with_capacity(objs.len());
                for arr_obj in objs {
                    array.push(T::try_from_object(doc, arr_obj)?);
                }
                Ok(array)
            }
            _ => bail!("unexpected object type"),
        }
    }
}

impl<'a> TryFromObject<'a> for Rectangle {
    fn try_from_object_direct(doc: &'a lopdf::Document, obj: &'a lopdf::Object) -> Fallible<Self>
    where
        Self: Sized,
    {
        match obj {
            lopdf::Object::Array(objs) => {
                if objs.len() != 4 {
                    bail!("wrong array length");
                }
                Ok(Rectangle {
                    lower_left_x: Number::try_from_object(doc, &objs[0])?.0,
                    lower_left_y: Number::try_from_object(doc, &objs[1])?.0,
                    upper_right_x: Number::try_from_object(doc, &objs[2])?.0,
                    upper_right_y: Number::try_from_object(doc, &objs[3])?.0,
                })
            }
            _ => bail!("unexpected object type"),
        }
    }
}
