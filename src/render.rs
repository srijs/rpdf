use std::sync::{Arc, Condvar, Mutex};

use crossbeam::thread;
use webrender::api::*;
use webrender::api::units::*;

use rpdf_document::Document;
use rpdf_render::DocumentRenderer;

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
    document: &'a Document,
    document_renderer: DocumentRenderer<'a>,
    api: RenderApi,
    state: Arc<(Mutex<Option<BackgroundRenderRequest>>, Condvar)>,
}

impl<'a> BackgroundRenderer<'a> {
    pub fn spawn<'scope>(
        scope: &'scope thread::Scope<'a>,
        document: &'a Document,
        api: RenderApi,
    ) -> BackgroundRendererRequestSender<'scope> {
        let mutex = Mutex::new(None);
        let condvar = Condvar::new();
        let state = Arc::new((mutex, condvar));
        let document_renderer = DocumentRenderer::new(document);

        let mut engine = Self {
            document,
            document_renderer,
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
        index: usize,
        scale: euclid::TypedScale<f32, LayoutPixel, LayoutPixel>,
        txn: &mut Transaction,
        document_id: DocumentId,
    ) -> (PipelineId, LayoutSize, BuiltDisplayList) {
        let page = &self.document.pages()[index];
        let size = LayoutSize::new(page.width() as f32, page.height() as f32);
        let scaled_size = scale.transform_size(&size);
        let page_pipeline_id = PipelineId(1, index as u32);
        let space_and_clip = SpaceAndClipInfo::root_scroll(page_pipeline_id);
        let mut builder = DisplayListBuilder::new(page_pipeline_id, size);
        let info = LayoutPrimitiveInfo::new(LayoutRect::new(LayoutPoint::zero(), scaled_size));
        builder.push_simple_stacking_context(
            &info,
            space_and_clip.spatial_id,
        );
        self.document_renderer.render_page(
            index as usize,
            scale,
            &self.api,
            &mut builder,
            txn,
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
        log::debug!("background render start");

        let total_width = self
            .document
            .pages()
            .iter()
            .map(|page| page.width() as i64)
            .max()
            .unwrap_or(0) as f32;

        let page_scale_factor = euclid::TypedScale::new((layout_size.width - 20.0) / total_width);

        let total_scaled_height = self
            .document
            .pages()
            .iter()
            .map(|page| page.height() as f32 * page_scale_factor.get() + 10.0)
            .sum::<f32>()
            + 10.0;

        for (index, page) in self.document.pages().iter().enumerate() {
            let size = LayoutSize::new(page.width() as f32, page.height() as f32);
            let mut txn = Transaction::new();
            let output = self.render_page(index, page_scale_factor, &mut txn, document_id);
            txn.set_display_list(Epoch(0), None, size, output, true);
            self.api.send_transaction(document_id, txn);
        }

        let space_and_clip = SpaceAndClipInfo::root_scroll(pipeline_id);
        let mut txn = webrender::api::Transaction::new();
        let mut builder = webrender::api::DisplayListBuilder::new(pipeline_id, layout_size);
        builder.push_rect(
            &LayoutPrimitiveInfo::new(LayoutRect::new(euclid::TypedPoint2D::zero(), layout_size)),
            &space_and_clip,
            ColorF::new(0.4, 0.4, 0.4, 1.0),
        );
        builder.push_simple_stacking_context(
            &webrender::api::LayoutPrimitiveInfo::new(LayoutRect::new(
                LayoutPoint::zero(),
                builder.content_size(),
            )),
            space_and_clip.spatial_id,
        );
        let scroll_space_and_clip = builder.define_scroll_frame(
            &space_and_clip,
            Some(ExternalScrollId(1, pipeline_id)),
            euclid::TypedRect::new(
                euclid::TypedPoint2D::zero(),
                euclid::TypedSize2D::new(layout_size.width, total_scaled_height),
            ),
            euclid::TypedRect::new(euclid::TypedPoint2D::zero(), layout_size),
            vec![],
            None,
            webrender::api::ScrollSensitivity::ScriptAndInputEvents,
            LayoutVector2D::zero(),
        );

        let mut info = webrender::api::LayoutPrimitiveInfo::new(LayoutRect::new(
            euclid::TypedPoint2D::zero(),
            euclid::TypedSize2D::new(layout_size.width, total_scaled_height),
        ));
        info.tag = Some((0, 1));
        builder.push_rect(
            &info,
            &scroll_space_and_clip,
            ColorF::new(0.4, 0.4, 0.4, 1.0),
        );

        let mut y = 0.0;
        for (index, page) in self.document.pages().iter().enumerate() {
            let page_size = LayoutSize::new(page.width() as f32, page.height() as f32);
            let scaled_page_size = page_scale_factor.transform_size(&page_size);

            builder.push_simple_stacking_context(
                &LayoutPrimitiveInfo::new(LayoutRect::new(
                    LayoutPoint::new(10.0, y + 10.0),
                    scaled_page_size,
                )),
                scroll_space_and_clip.spatial_id,
            );
            builder.push_shadow(
                &LayoutPrimitiveInfo::new(LayoutRect::new(LayoutPoint::zero(), scaled_page_size)),
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
                    scaled_page_size,
                )),
                &scroll_space_and_clip,
                ColorF::WHITE,
            );
            builder.pop_all_shadows();
            builder.push_iframe(
                &LayoutPrimitiveInfo::new(LayoutRect::new(LayoutPoint::zero(), scaled_page_size)),
                &scroll_space_and_clip,
                PipelineId(1, index as u32),
                true,
            );
            builder.pop_stacking_context();
            y += 10.0 + scaled_page_size.height;
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
