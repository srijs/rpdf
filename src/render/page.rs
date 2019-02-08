use std::collections::HashMap;

use webrender::api::*;

use crate::pdf;

pub struct PageRenderer<'a> {
    page: &'a pdf::Page,
    font_keys: HashMap<&'a [u8], Option<FontKey>>,
    font_instance_keys: HashMap<(&'a [u8], u32), FontInstanceKey>,
}

impl<'a> PageRenderer<'a> {
    pub fn new(page: &'a pdf::Page) -> Self {
        Self {
            page,
            font_keys: HashMap::new(),
            font_instance_keys: HashMap::new(),
        }
    }

    fn load_font(
        &mut self,
        api: &RenderApi,
        txn: &mut Transaction,
        name: &'a [u8],
    ) -> Option<FontKey> {
        let page = self.page;
        *self.font_keys.entry(name).or_insert_with(|| {
            let key = api.generate_font_key();
            if let Some(font) = page.font(name) {
                txn.add_raw_font(key, font.data().to_owned(), 0);
                Some(key)
            } else {
                None
            }
        })
    }

    fn load_font_instance(
        &mut self,
        api: &RenderApi,
        txn: &mut Transaction,
        name: &'a [u8],
        size: u32,
    ) -> FontInstanceKey {
        let font_keys = &self.font_keys;
        *self
            .font_instance_keys
            .entry((name, size))
            .or_insert_with(|| {
                let key = api.generate_font_instance_key();
                let font_key = font_keys[name].unwrap();
                txn.add_font_instance(
                    key,
                    font_key,
                    app_units::Au::from_px(size as i32),
                    None,
                    None,
                    vec![],
                );
                key
            })
    }

    pub fn render(
        &mut self,
        api: &RenderApi,
        builder: &mut DisplayListBuilder,
        txn: &mut Transaction,
        pipeline_id: PipelineId,
        document_id: DocumentId,
        space_and_clip: &SpaceAndClipInfo,
    ) {
        for text_object in self.page.text() {
            for text_fragment in text_object.fragments.iter() {
                let mut transform =
                    euclid::TypedTransform2D::from_untyped(&text_fragment.transform);
                transform.m32 = self.page.height() as f32 - transform.m32;

                if self.load_font(api, txn, &text_fragment.font_name).is_none() {
                    // skip text fragments that don't have font data
                    continue;
                };

                let font_instance_key = self.load_font_instance(
                    api,
                    txn,
                    &text_fragment.font_name,
                    text_fragment.font_size as u32,
                );

                let mut glyph_instances = Vec::with_capacity(text_fragment.glyphs.len());

                let mut x = 0.0;
                for text_glyph in text_fragment.glyphs.iter() {
                    let mut point = euclid::TypedPoint2D::from_untyped(&text_glyph.origin);
                    point.y = self.page.height() as f32 - point.y;
                    glyph_instances.push(GlyphInstance {
                        index: text_glyph.index,
                        point: point,
                    });
                    x += text_glyph.advance;
                }

                let size = euclid::TypedSize2D::<f32, LayoutPixel>::new(x, 60.0);
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
