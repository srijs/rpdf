use std::collections::HashMap;

use app_units::Au;
use webrender::api::*;

use rpdf_document::Font;

#[derive(Default)]
pub struct FontRenderContext<'a> {
    font_keys: HashMap<&'a [u8], FontKey>,
    font_instance_keys: HashMap<(&'a [u8], Au), FontInstanceKey>,
}

impl<'a> FontRenderContext<'a> {
    pub fn load_font(
        &mut self,
        api: &RenderApi,
        txn: &mut Transaction,
        name: &'a [u8],
        font: &Font,
    ) -> FontKey {
        *self.font_keys.entry(name).or_insert_with(|| {
            let key = api.generate_font_key();
            txn.add_raw_font(key, font.data().to_owned(), 0);
            key
        })
    }

    pub fn load_font_instance(
        &mut self,
        api: &RenderApi,
        txn: &mut Transaction,
        name: &'a [u8],
        size: f32,
    ) -> FontInstanceKey {
        let au = Au::from_f32_px(size);
        let font_keys = &self.font_keys;
        *self
            .font_instance_keys
            .entry((name, au))
            .or_insert_with(|| {
                let key = api.generate_font_instance_key();
                let font_key = font_keys[name];
                txn.add_font_instance(key, font_key, au, None, None, vec![]);
                key
            })
    }
}
