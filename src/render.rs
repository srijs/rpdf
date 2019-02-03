use std::sync::{Arc, Condvar, Mutex};

use crossbeam::thread;
use webrender::api::*;

use crate::pdf;

mod page;
use self::page::PageRenderer;

enum BackgroundRenderRequest {
    Render {
        epoch: Epoch,
        pipeline_id: PipelineId,
        document_id: DocumentId,
        layout_size: LayoutSize,
    },
    Shutdown,
}

pub struct BackgroundRendererRequestSender<'a> {
    state: Arc<(Mutex<Option<BackgroundRenderRequest>>, Condvar)>,
    join_handle: thread::ScopedJoinHandle<'a, ()>,
}

impl<'a> BackgroundRendererRequestSender<'a> {
    pub fn render(
        &self,
        epoch: Epoch,
        pipeline_id: PipelineId,
        document_id: DocumentId,
        layout_size: LayoutSize,
    ) {
        log::debug!("background render requested");
        self.send(BackgroundRenderRequest::Render {
            epoch,
            pipeline_id,
            document_id,
            layout_size,
        });
    }

    pub fn shutdown(self) {
        log::debug!("background shutdown requested");
        self.send(BackgroundRenderRequest::Shutdown);
        self.join_handle.join().unwrap();
    }

    fn send(&self, req: BackgroundRenderRequest) {
        let mut lock = self.state.0.lock().unwrap();
        *lock = Some(req);
        drop(lock);
        self.state.1.notify_one();
    }
}

pub struct BackgroundRenderer<'a> {
    page_renderers: Vec<(LayoutSize, PageRenderer<'a>)>,
    api: RenderApi,
    state: Arc<(Mutex<Option<BackgroundRenderRequest>>, Condvar)>,
}

impl<'a> BackgroundRenderer<'a> {
    pub fn spawn<'scope>(
        scope: &'scope thread::Scope<'a>,
        pages: &'a [pdf::Page],
        api: RenderApi,
    ) -> BackgroundRendererRequestSender<'scope> {
        let mutex = Mutex::new(None);
        let condvar = Condvar::new();
        let state = Arc::new((mutex, condvar));
        let page_renderers = pages
            .iter()
            .map(|page| {
                let size = LayoutSize::new(page.width() as f32, page.height() as f32);
                (size, PageRenderer::new(page))
            })
            .collect::<Vec<_>>();

        let mut engine = Self {
            page_renderers,
            api,
            state: state.clone(),
        };

        let join_handle = scope.spawn(move |_| loop {
            let req = {
                let mut lock = engine.state.0.lock().unwrap();
                loop {
                    if let Some(req) = lock.take() {
                        break req;
                    }
                    lock = engine.state.1.wait(lock).unwrap();
                }
            };
            match req {
                BackgroundRenderRequest::Render {
                    epoch,
                    pipeline_id,
                    document_id,
                    layout_size,
                } => {
                    engine.render(epoch, pipeline_id, document_id, layout_size);
                }
                BackgroundRenderRequest::Shutdown => {
                    break;
                }
            }
        });

        BackgroundRendererRequestSender { state, join_handle }
    }

    fn render_page(
        &mut self,
        index: u32,
        txn: &mut Transaction,
        document_id: DocumentId,
    ) -> (PipelineId, LayoutSize, BuiltDisplayList) {
        let (size, ref mut page_renderer) = self.page_renderers[index as usize];
        let page_pipeline_id = PipelineId(1, index);
        let space_and_clip = SpaceAndClipInfo::root_scroll(page_pipeline_id);
        let mut builder = DisplayListBuilder::new(page_pipeline_id, size);
        let info = LayoutPrimitiveInfo::new(LayoutRect::new(LayoutPoint::zero(), size));
        builder.push_stacking_context(
            &info,
            space_and_clip.spatial_id,
            None,
            webrender::api::TransformStyle::Flat,
            webrender::api::MixBlendMode::Normal,
            &[],
            webrender::api::RasterSpace::Screen,
            false,
        );
        page_renderer.render(
            &self.api,
            &mut builder,
            txn,
            page_pipeline_id,
            document_id,
            &space_and_clip,
        );
        builder.pop_stacking_context();
        builder.finalize()
    }

    fn render(
        &mut self,
        epoch: Epoch,
        pipeline_id: PipelineId,
        document_id: DocumentId,
        layout_size: LayoutSize,
    ) {
        let space_and_clip = SpaceAndClipInfo::root_scroll(pipeline_id);
        log::debug!("background render start");

        for i in 0..self.page_renderers.len() {
            let mut txn = Transaction::new();
            let output = self.render_page(i as u32, &mut txn, document_id);
            txn.set_display_list(Epoch(0), None, self.page_renderers[i].0, output, true);
            self.api.send_transaction(document_id, txn);
        }

        let total_width = self
            .page_renderers
            .iter()
            .map(|(size, _)| size.width as i64)
            .max()
            .unwrap_or(0) as f32;
        let total_height = self
            .page_renderers
            .iter()
            .map(|(size, _)| size.height + 10.0)
            .sum::<f32>()
            + 10.0;

        let mut horizontal_padding = (layout_size.width - total_width) / 2.0;
        if horizontal_padding < 0.0 {
            horizontal_padding = 0.0;
        }

        let mut txn = webrender::api::Transaction::new();
        let mut builder = webrender::api::DisplayListBuilder::new(pipeline_id, layout_size);
        builder.push_rect(
            &LayoutPrimitiveInfo::new(LayoutRect::new(euclid::TypedPoint2D::zero(), layout_size)),
            &space_and_clip,
            ColorF::new(0.4, 0.4, 0.4, 1.0),
        );
        builder.push_stacking_context(
            &webrender::api::LayoutPrimitiveInfo::new(webrender::api::LayoutRect::new(
                webrender::api::LayoutPoint::zero(),
                builder.content_size(),
            )),
            space_and_clip.spatial_id,
            None,
            webrender::api::TransformStyle::Flat,
            webrender::api::MixBlendMode::Normal,
            &[],
            webrender::api::RasterSpace::Screen,
            false,
        );
        let scroll_space_and_clip = builder.define_scroll_frame(
            &space_and_clip,
            None,
            euclid::TypedRect::new(
                euclid::TypedPoint2D::new(horizontal_padding, 0.0),
                euclid::TypedSize2D::new(total_width, total_height),
            ),
            euclid::TypedRect::new(euclid::TypedPoint2D::zero(), layout_size),
            vec![],
            None,
            webrender::api::ScrollSensitivity::ScriptAndInputEvents,
        );

        let mut info = webrender::api::LayoutPrimitiveInfo::new(webrender::api::LayoutRect::new(
            euclid::TypedPoint2D::zero(),
            euclid::TypedSize2D::new(total_width, total_height),
        ));
        info.tag = Some((0, 1));
        builder.push_rect(
            &info,
            &scroll_space_and_clip,
            ColorF::new(0.4, 0.4, 0.4, 1.0),
        );

        let mut y = 0.0;
        for (i, (page_size, _)) in self.page_renderers.iter_mut().enumerate() {
            builder.push_stacking_context(
                &LayoutPrimitiveInfo::new(LayoutRect::new(
                    LayoutPoint::new(horizontal_padding, y + 10.0),
                    *page_size,
                )),
                scroll_space_and_clip.spatial_id,
                None,
                TransformStyle::Flat,
                MixBlendMode::Normal,
                &[],
                RasterSpace::Screen,
                false,
            );
            builder.push_shadow(
                &LayoutPrimitiveInfo::new(LayoutRect::new(LayoutPoint::zero(), *page_size)),
                &scroll_space_and_clip,
                Shadow {
                    offset: LayoutVector2D::zero(),
                    color: ColorF::BLACK,
                    blur_radius: 5.0,
                    should_inflate: true,
                },
            );
            builder.push_rect(
                &LayoutPrimitiveInfo::new(LayoutRect::new(
                    euclid::TypedPoint2D::zero(),
                    *page_size,
                )),
                &scroll_space_and_clip,
                ColorF::WHITE,
            );
            builder.pop_all_shadows();
            builder.push_iframe(
                &LayoutPrimitiveInfo::new(LayoutRect::new(LayoutPoint::zero(), *page_size)),
                &scroll_space_and_clip,
                PipelineId(1, i as u32),
                true,
            );
            builder.pop_stacking_context();
            y += 10.0 + page_size.height;
        }

        builder.pop_stacking_context();
        txn.set_display_list(
            epoch,
            Some(webrender::api::ColorF::new(1.0, 1.0, 1.0, 1.0)),
            layout_size,
            builder.finalize(),
            true,
        );
        txn.generate_frame();
        self.api.send_transaction(document_id, txn);
        log::debug!("background render finish");
    }
}
