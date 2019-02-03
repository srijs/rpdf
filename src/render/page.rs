use std::collections::HashMap;

use webrender::api::*;

use crate::pdf;

pub struct PageRenderer<'a> {
    page: &'a pdf::Page,
    font_keys: HashMap<&'a [u8], FontKey>,
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

    fn load_font(&mut self, api: &RenderApi, txn: &mut Transaction, name: &'a [u8]) -> FontKey {
        let page = self.page;
        *self.font_keys.entry(name).or_insert_with(|| {
            let key = api.generate_font_key();
            let font = page.font(name).unwrap();
            match font.data().unwrap() {
                pdf::FontData::Type1(bytes) => {
                    txn.add_raw_font(key, (**bytes).clone(), 0);
                }
            }
            key
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
                let font_key = font_keys[name];
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

                let font_key = self.load_font(api, txn, &text_fragment.font_name);
                let font_instance_key = self.load_font_instance(
                    api,
                    txn,
                    &text_fragment.font_name,
                    text_fragment.font_size as u32,
                );

                let mut glyph_instances = Vec::with_capacity(text_fragment.glyphs.len());
                let mut text = String::new();

                let mut x = 0.0;
                for text_glyph in text_fragment.glyphs.iter() {
                    text.push(text_glyph.code);
                    let mut point = euclid::TypedPoint2D::from_untyped(&text_glyph.origin);
                    point.y = self.page.height() as f32 - point.y;
                    glyph_instances.push(GlyphInstance {
                        index: 0,
                        point: point,
                    });
                    x += text_glyph.advance;
                }

                let glyph_indices = api.get_glyph_indices(font_key, &text);
                for (i, glyph_index) in glyph_indices.iter().enumerate() {
                    glyph_instances[i].index = glyph_index.unwrap_or(0);
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
