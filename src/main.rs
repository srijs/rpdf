use std::fs::File;
use std::path::PathBuf;

use crossbeam::thread;
use failure::Fallible;
use glutin::GlContext;
use structopt::StructOpt;
use webrender::api::units::*;

use rpdf_document::Document;

mod render;

#[derive(Debug, StructOpt)]
#[structopt(name = "rpdf")]
struct Opt {
    #[structopt(parse(from_os_str))]
    input: PathBuf,
}

fn render<'env>(scope: &thread::Scope<'env>, document: &'env Document) -> Fallible<()> {
    let pages = document.pages();
    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new()
        .with_title("rPDF")
        .with_dimensions((pages[0].width() + 20.0, pages[0].height()).into());
    let context = glutin::ContextBuilder::new()
        .with_vsync(false)
        .with_multisampling(4)
        .with_srgb(true);
    let gl_window = glutin::GlWindow::new(window, context, &events_loop).unwrap();

    unsafe {
        gl_window.make_current().unwrap();
    }

    let gl = match gl_window.get_api() {
        glutin::Api::OpenGl => unsafe {
            gleam::gl::GlFns::load_with(|symbol| gl_window.get_proc_address(symbol) as *const _)
        },
        glutin::Api::OpenGlEs => unsafe {
            gleam::gl::GlesFns::load_with(|symbol| gl_window.get_proc_address(symbol) as *const _)
        },
        glutin::Api::WebGl => unimplemented!(),
    };

    let mut device_pixel_ratio = gl_window.get_hidpi_factor();

    let opts = webrender::RendererOptions {
        device_pixel_ratio: device_pixel_ratio as f32,
        ..webrender::RendererOptions::default()
    };

    let mut framebuffer_size = {
        let size = gl_window
            .get_inner_size()
            .unwrap()
            .to_physical(device_pixel_ratio);
        FramebufferIntSize::new(size.width as i32, size.height as i32)
    };

    let notifier = Box::new(Notifier::new(events_loop.create_proxy()));

    let (mut renderer, sender) =
        webrender::Renderer::new(gl.clone(), notifier, opts, None, framebuffer_size).unwrap();
    let api = sender.create_api();
    let bgapi = sender.create_api();
    let document_id = api.add_document(framebuffer_size, 0);

    let mut epoch = webrender::api::Epoch(0);
    let pipeline_id = webrender::api::PipelineId(0, 0);
    let layout_size =
        framebuffer_size.to_f32() / euclid::TypedScale::new(device_pixel_ratio as f32);

    let mut txn = webrender::api::Transaction::new();
    txn.set_root_pipeline(pipeline_id);
    txn.generate_frame();
    api.send_transaction(document_id, txn);

    let background = render::BackgroundRenderer::spawn(scope, document, bgapi);
    background.render(epoch, pipeline_id, document_id, layout_size);

    // Stores the currently known cursor position.
    let mut cursor_position = WorldPoint::zero();

    // Indicates whether the OpenGL window should be redrawn.
    let mut needs_repaint = true;

    events_loop.run_forever(|event| {
        // Indicates whether the display layout should be recalculated.
        let mut needs_render = false;

        match event {
            glutin::Event::Awakened => {
                needs_repaint = true;
            }
            glutin::Event::WindowEvent { event, .. } => match event {
                glutin::WindowEvent::CloseRequested => {
                    return glutin::ControlFlow::Break;
                }
                glutin::WindowEvent::Resized(size) => {
                    gl_window.resize(size.to_physical(device_pixel_ratio));
                    framebuffer_size = {
                        let size = gl_window
                            .get_inner_size()
                            .unwrap()
                            .to_physical(device_pixel_ratio);
                        FramebufferIntSize::new(size.width as i32, size.height as i32)
                    };
                    api.set_document_view(
                        document_id,
                        FramebufferIntRect::new(FramebufferIntPoint::zero(), framebuffer_size),
                        device_pixel_ratio as f32,
                    );
                    needs_render = true;
                    needs_repaint = true;
                }
                glutin::WindowEvent::HiDpiFactorChanged(factor) => {
                    device_pixel_ratio = factor;
                    framebuffer_size = {
                        let size = gl_window
                            .get_inner_size()
                            .unwrap()
                            .to_physical(device_pixel_ratio);
                        FramebufferIntSize::new(size.width as i32, size.height as i32)
                    };
                    needs_render = true;
                    needs_repaint = true;
                }
                glutin::WindowEvent::Refresh => {
                    needs_repaint = true;
                }
                glutin::WindowEvent::CursorMoved {
                    position: glutin::dpi::LogicalPosition { x, y },
                    ..
                } => {
                    cursor_position = WorldPoint::new(x as f32, y as f32);
                    return glutin::ControlFlow::Continue;
                }
                glutin::WindowEvent::MouseWheel { delta, .. } => {
                    const LINE_HEIGHT: f32 = 38.0;
                    let (dx, dy) = match delta {
                        glutin::MouseScrollDelta::LineDelta(dx, dy) => (dx, dy * LINE_HEIGHT),
                        glutin::MouseScrollDelta::PixelDelta(pos) => (pos.x as f32, pos.y as f32),
                    };

                    let mut txn = webrender::api::Transaction::new();
                    txn.scroll(
                        webrender::api::ScrollLocation::Delta(LayoutVector2D::new(dx, dy)),
                        cursor_position,
                    );
                    txn.generate_frame();
                    api.send_transaction(document_id, txn);
                }
                _ => {
                    return glutin::ControlFlow::Continue;
                }
            },
            _ => {
                return glutin::ControlFlow::Continue;
            }
        }

        if needs_repaint {
            renderer.update();
            renderer.render(framebuffer_size).unwrap();
            let _ = renderer.flush_pipeline_info();
            gl_window.swap_buffers().unwrap();
        }

        let layout_size =
            framebuffer_size.to_f32() / euclid::TypedScale::new(device_pixel_ratio as f32);

        if needs_render {
            epoch = webrender::api::Epoch(epoch.0 + 1);
            background.render(epoch, pipeline_id, document_id, layout_size);
        }

        glutin::ControlFlow::Continue
    });

    background.shutdown();
    renderer.deinit();

    Ok(())
}

fn main() -> Fallible<()> {
    env_logger::init();

    let opt = Opt::from_args();

    let input_file = File::open(&opt.input)?;
    let document = Document::parse(input_file)?;

    thread::scope(|scope| render(scope, &document)).unwrap()?;

    Ok(())
}

struct Notifier {
    events_proxy: glutin::EventsLoopProxy,
}

impl Notifier {
    fn new(events_proxy: glutin::EventsLoopProxy) -> Notifier {
        Notifier { events_proxy }
    }
}

impl webrender::api::RenderNotifier for Notifier {
    fn clone(&self) -> Box<webrender::api::RenderNotifier> {
        Box::new(Notifier {
            events_proxy: self.events_proxy.clone(),
        })
    }

    fn wake_up(&self) {
        self.events_proxy.wakeup().ok();
    }

    fn new_frame_ready(
        &self,
        _: webrender::api::DocumentId,
        _scrolled: bool,
        _composite_needed: bool,
        _render_time: Option<u64>,
    ) {
        self.wake_up();
    }
}
