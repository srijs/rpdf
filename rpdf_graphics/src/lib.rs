use std::sync::Arc;

use failure::Fallible;

pub mod data;
pub mod font;
pub mod text;

const OP_BEGIN_TEXT_OBJECT: &str = "BT";
const OP_END_TEXT_OBJECT: &str = "ET";

pub enum GraphicsObject {
    Text(text::TextObject),
}

pub struct GraphicsState {
    text_state: text::TextState,
}

impl GraphicsState {
    fn new() -> Self {
        GraphicsState {
            text_state: text::TextState::default(),
        }
    }
}

enum GraphicsObjectBuilder<'a> {
    Text(text::TextObjectBuilder<'a>),
}

pub struct GraphicsObjectDecoder<'a> {
    document: Arc<lopdf::Document>,
    font_map: &'a font::FontMap,
    operations: std::vec::IntoIter<lopdf::content::Operation>,
    state: GraphicsState,
    builder: Option<GraphicsObjectBuilder<'a>>,
}

impl<'a> GraphicsObjectDecoder<'a> {
    pub fn decode(
        document: Arc<lopdf::Document>,
        font_map: &'a font::FontMap,
        data: &[u8],
    ) -> Fallible<Self> {
        let content = lopdf::content::Content::decode(data)?;
        Ok(Self {
            document,
            font_map,
            operations: content.operations.into_iter(),
            state: GraphicsState::new(),
            builder: None,
        })
    }

    fn try_next(&mut self) -> Fallible<Option<GraphicsObject>> {
        loop {
            if let Some(operation) = self.operations.next() {
                match operation.operator.as_str() {
                    OP_BEGIN_TEXT_OBJECT => {
                        let text_builder =
                            text::TextObjectBuilder::new(self.document.clone(), self.font_map);
                        self.builder = Some(GraphicsObjectBuilder::Text(text_builder));
                    }
                    OP_END_TEXT_OBJECT => {
                        if let Some(GraphicsObjectBuilder::Text(text_builder)) = self.builder.take()
                        {
                            return Ok(Some(GraphicsObject::Text(text_builder.build())));
                        } else {
                            failure::bail!("state transition error");
                        }
                    }
                    "Tc" | "Tw" | "Tz" | "TL" | "Tf" => {
                        self.state
                            .text_state
                            .handle_operation(&self.document, &operation)?;
                    }
                    _ => match self.builder {
                        Some(GraphicsObjectBuilder::Text(ref mut text_builder)) => {
                            text_builder
                                .handle_operation(&mut self.state.text_state, &operation)?;
                        }
                        None => {}
                    },
                }
            } else {
                return Ok(None);
            }
        }
    }
}

impl<'a> Iterator for GraphicsObjectDecoder<'a> {
    type Item = Fallible<GraphicsObject>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.try_next() {
            Ok(Some(object)) => Some(Ok(object)),
            Ok(None) => None,
            Err(err) => Some(Err(err)),
        }
    }
}
