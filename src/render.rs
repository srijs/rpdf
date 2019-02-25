use webrender::api::*;

use crate::pdf;

mod text;
use self::text::FontRenderContext;
mod page;
use self::page::PageRenderer;

pub struct DocumentRenderer<'a> {
    document: &'a pdf::Document,
    page_renderers: Vec<PageRenderer<'a>>,
    font_context: FontRenderContext<'a>,
}

impl<'a> DocumentRenderer<'a> {
    pub fn new(document: &'a pdf::Document) -> Self {
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
        api: &RenderApi,
        builder: &mut DisplayListBuilder,
        txn: &mut Transaction,
        space_and_clip: &SpaceAndClipInfo,
    ) {
        self.page_renderers[index].render(api, builder, txn, space_and_clip, &mut self.font_context)
    }
}
