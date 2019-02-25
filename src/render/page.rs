use webrender::api::*;

use crate::pdf;

use super::text::FontRenderContext;

pub struct PageRenderer<'a> {
    page: &'a pdf::Page,
}

impl<'a> PageRenderer<'a> {
    pub fn new(page: &'a pdf::Page) -> Self {
        Self { page }
    }

    pub fn render(
        &mut self,
        api: &RenderApi,
        builder: &mut DisplayListBuilder,
        txn: &mut Transaction,
        space_and_clip: &SpaceAndClipInfo,
        font_context: &mut FontRenderContext<'a>,
    ) {
        for text_object in self.page.text() {
            for text_fragment in text_object.fragments.iter() {
                let mut transform =
                    euclid::TypedTransform2D::from_untyped(&text_fragment.transform);
                transform.m32 = self.page.height() as f32 - transform.m32;

                if let Some(font) = self.page.font(&text_fragment.font_name) {
                    font_context.load_font(api, txn, &text_fragment.font_name, font);
                } else {
                    // skip text fragments that don't have font data
                    continue;
                };

                let font_size = text_fragment.font_size;
                let font_instance_key =
                    font_context.load_font_instance(api, txn, &text_fragment.font_name, font_size);

                let mut glyph_instances = Vec::with_capacity(text_fragment.glyphs.len());

                for text_glyph in text_fragment.glyphs.iter() {
                    let mut point = euclid::TypedPoint2D::from_untyped(&text_glyph.origin);
                    point.y = self.page.height() as f32 - point.y;
                    glyph_instances.push(GlyphInstance {
                        index: text_glyph.index,
                        point: point,
                    });
                }

                let size =
                    euclid::TypedSize2D::<f32, LayoutPixel>::new(self.page.width() as f32, 60.0);
                let rect = euclid::TypedRect::<f32, LayoutPixel>::new(
                    euclid::TypedPoint2D::<f32, LayoutPixel>::new(0.0, -30.0),
                    size,
                );
                let transformed_rect = transform.transform_rect(&rect);

                log::trace!("push text {:?} {:?}", glyph_instances, transformed_rect);

                builder.push_text(
                    &LayoutPrimitiveInfo::new(transformed_rect),
                    space_and_clip,
                    &glyph_instances,
                    font_instance_key,
                    ColorF::BLACK,
                    None,
                );
            }
        }
    }
}
