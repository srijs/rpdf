use webrender::api::*;

use rpdf_document::Document;

mod text;
use self::text::FontRenderContext;
mod page;
use self::page::PageRenderer;

pub struct DocumentRenderer<'a> {
    document: &'a Document,
    page_renderers: Vec<PageRenderer<'a>>,
    font_context: FontRenderContext<'a>,
}

impl<'a> DocumentRenderer<'a> {
    pub fn new(document: &'a Document) -> Self {
        let page_renderers = document
            .pages()
            .iter()
            .map(|page| PageRenderer::new(page))
            .collect();

        Self {
            document,
            page_renderers,
            font_context: Default::default(),
        }
    }

    pub fn render_page(
        &mut self,
        index: usize,
        scale: euclid::TypedScale<f32, LayoutPixel, LayoutPixel>,
        api: &RenderApi,
        builder: &mut DisplayListBuilder,
        txn: &mut Transaction,
        space_and_clip: &SpaceAndClipInfo,
    ) {
        self.page_renderers[index].render(
            scale,
            api,
            builder,
            txn,
            space_and_clip,
            &mut self.font_context,
        )
    }
}
