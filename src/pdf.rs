use std::io;
use std::sync::Arc;

use failure::Fallible;

mod data;
mod font;
mod text;

use self::data::TryFromObject;
pub use self::font::{Font, FontMap};
pub use self::text::{TextFragment, TextObject};

pub struct Document {
    inner: Arc<lopdf::Document>,
    pages: Vec<Page>,
}

impl Document {
    pub fn parse<R>(reader: R) -> Fallible<Self>
    where
        R: io::Read,
    {
        let document = Arc::new(lopdf::Document::load_from(reader)?);
        let pages = document
            .get_pages()
            .values()
            .map(|object_id| {
                let page_dict = document.get_dictionary(*object_id).unwrap();
                let media_box = data::Rectangle::try_from_object(
                    &document,
                    page_dict.get(b"MediaBox").unwrap(),
                )?;
                let content = document.get_page_content(*object_id)?;
                let font_map =
                    FontMap::try_from_page_fonts(&document, document.get_page_fonts(*object_id))?;
                let text_objects =
                    text::TextIter::decode(document.clone(), &font_map, &content)?.collect();

                Ok(Page {
                    document: document.clone(),
                    object_id: *object_id,
                    media_box,
                    text_objects,
                    font_map,
                })
            })
            .collect::<Fallible<Vec<Page>>>()?;
        Ok(Self {
            inner: document,
            pages,
        })
    }

    pub fn pages(&self) -> &[Page] {
        &self.pages
    }
}

pub struct Page {
    document: Arc<lopdf::Document>,
    object_id: lopdf::ObjectId,
    media_box: data::Rectangle,
    text_objects: Vec<TextObject>,
    font_map: FontMap,
}

impl Page {
    pub fn width(&self) -> f64 {
        self.media_box.width()
    }

    pub fn height(&self) -> f64 {
        self.media_box.height()
    }

    pub fn text(&self) -> &[TextObject] {
        &self.text_objects
    }

    pub fn font(&self, name: &[u8]) -> Option<&Font> {
        self.font_map.get(name)
    }
}
