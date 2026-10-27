//! The video surface: libmpv drawing into a `GtkGLArea` through mpv's
//! render API. Platform glue only; what to open comes from core as an
//! [`Open`].
//!
//! The app has one player, so one [`Mpv`] lives for the whole process:
//! mpv's render context borrows its `Mpv`, and a `'static` one lets the
//! context sit in the widget. The render context follows the GL area's
//! own context: made on realize, dropped on unrealize.

use std::cell::{Cell, RefCell};
use std::ffi::{CString, c_char, c_void};
use std::rc::Rc;

use gtk::glib;
use gtk::prelude::*;
use jitter_core::models::{Open, Seconds};
use libmpv2::Mpv;
use libmpv2::render::{OpenGLInitParams, RenderContext, RenderParam, RenderParamApiType};

/// `GL_FRAMEBUFFER_BINDING`: the framebuffer GTK bound for the area.
const FRAMEBUFFER_BINDING: u32 = 0x8CA6;

type GetProcAddress = unsafe extern "C" fn(*const c_char) -> *mut c_void;
type GetIntegerv = unsafe extern "C" fn(u32, *mut i32);

#[derive(Clone)]
pub struct Video {
    area: gtk::GLArea,
    mpv: &'static Mpv,
}

impl Video {
    pub fn new() -> Result<Self, String> {
        let mpv: &'static Mpv = Box::leak(Box::new(start_mpv()?));
        let area = gtk::GLArea::builder().hexpand(true).vexpand(true).build();
        let render: Rc<RefCell<Option<RenderContext<'static>>>> = Rc::default();
        let get_integerv: Rc<Cell<Option<GetIntegerv>>> = Rc::default();
        let (redraw, redraws) = async_channel::unbounded::<()>();

        let (on_realize, integerv) = (render.clone(), get_integerv.clone());
        area.connect_realize(move |area| {
            area.make_current();
            if let Some(e) = area.error() {
                eprintln!("jitter: the video surface has no GL: {e}");
                return;
            }
            let loader = match GlLoader::load() {
                Ok(loader) => loader,
                Err(e) => return eprintln!("jitter: loading GL: {e}"),
            };
            integerv.set(loader.get_integerv());
            let created = mpv.create_render_context(vec![
                RenderParam::ApiType(RenderParamApiType::OpenGl),
                RenderParam::InitParams(OpenGLInitParams {
                    get_proc_address: GlLoader::proc_address,
                    ctx: loader,
                }),
            ]);
            match created {
                Ok(mut context) => {
                    let redraw = redraw.clone();
                    // Runs on mpv's thread: only wake the main loop.
                    context.set_update_callback(move || {
                        let _ = redraw.try_send(());
                    });
                    on_realize.replace(Some(context));
                }
                Err(e) => eprintln!("jitter: starting mpv's renderer: {e}"),
            }
        });

        let on_unrealize = render.clone();
        area.connect_unrealize(move |area| {
            area.make_current();
            on_unrealize.replace(None);
        });

        let on_render = render.clone();
        area.connect_render(move |area, _| {
            if let (Some(context), Some(get_integerv)) =
                (on_render.borrow().as_ref(), get_integerv.get())
            {
                let mut fbo = 0;
                // SAFETY: GTK made the area's GL context current for this
                // signal, and `get_integerv` came from that GL.
                unsafe { get_integerv(FRAMEBUFFER_BINDING, &mut fbo) };
                let scale = area.scale_factor();
                let (width, height) = (area.width() * scale, area.height() * scale);
                if let Err(e) = context.render::<GlLoader>(fbo, width, height, true) {
                    eprintln!("jitter: drawing a frame: {e}");
                }
            }
            glib::Propagation::Stop
        });

        let weak = area.downgrade();
        glib::spawn_future_local(async move {
            while redraws.recv().await.is_ok() {
                let Some(area) = weak.upgrade() else { break };
                let frame = render
                    .borrow()
                    .as_ref()
                    .and_then(|context| context.update().ok())
                    .is_some_and(|flags| {
                        u64::from(flags) & u64::from(libmpv2::render::mpv_render_update::Frame) != 0
                    });
                if frame {
                    area.queue_render();
                }
            }
        });

        Ok(Self { area, mpv })
    }

    pub fn widget(&self) -> &gtk::GLArea {
        &self.area
    }

    /// Opens `open.url` where core says to start.
    pub fn open(&self, open: &Open) {
        let start = match open.live {
            true => "none".to_owned(),
            false => open.start.get().to_string(),
        };
        let result = self
            .mpv
            .set_property("start", start)
            .and_then(|()| self.mpv.command("loadfile", &[&open.url, "replace"]));
        if let Err(e) = result {
            eprintln!("jitter: opening the stream: {e}");
        }
    }

    pub fn stop(&self) {
        if let Err(e) = self.mpv.command("stop", &[]) {
            eprintln!("jitter: stopping the player: {e}");
        }
    }

    /// Where the player is; zero before anything plays.
    pub fn position(&self) -> Seconds {
        let at = self.mpv.get_property::<f64>("time-pos").unwrap_or(0.0);
        // Saturating: a negative or huge position clamps into range.
        Seconds::new(at.max(0.0) as u32)
    }
}

fn start_mpv() -> Result<Mpv, String> {
    // libmpv refuses to start unless numbers parse the C way, and GTK set
    // the user's locale when it started.
    // SAFETY: called on the main thread before any other thread reads
    // the locale.
    unsafe { libc::setlocale(libc::LC_NUMERIC, c"C".as_ptr()) };
    Mpv::with_initializer(|init| {
        init.set_property("vo", "libmpv")?;
        init.set_property("hwdec", "auto-safe")?;
        init.set_property("keep-open", "yes")?;
        Ok(())
    })
    .map_err(|e| format!("starting mpv: {e}"))
}

/// Resolves GL functions for mpv through the windowing system's loader:
/// EGL, which GTK 4 draws with, or GLX on an X server without it.
struct GlLoader {
    get: GetProcAddress,
    _library: libloading::Library,
}

impl GlLoader {
    fn load() -> Result<Self, libloading::Error> {
        Self::open("libEGL.so.1", b"eglGetProcAddress\0")
            .or_else(|_| Self::open("libGL.so.1", b"glXGetProcAddressARB\0"))
    }

    fn open(library: &str, symbol: &[u8]) -> Result<Self, libloading::Error> {
        // SAFETY: the system GL loaders run no code on load that relies on
        // anything but the process being started, and the symbol has the
        // `GetProcAddress` signature in both.
        unsafe {
            let library = libloading::Library::new(library)?;
            let get = *library.get::<GetProcAddress>(symbol)?;
            Ok(Self {
                get,
                _library: library,
            })
        }
    }

    fn proc_address(&self, name: &str) -> *mut c_void {
        let Ok(name) = CString::new(name) else {
            return std::ptr::null_mut();
        };
        // SAFETY: `name` is a NUL-terminated string that outlives the call.
        unsafe { (self.get)(name.as_ptr()) }
    }

    fn get_integerv(&self) -> Option<GetIntegerv> {
        let address = self.proc_address("glGetIntegerv");
        // SAFETY: a non-null `glGetIntegerv` has the `GetIntegerv` signature.
        (!address.is_null())
            .then(|| unsafe { std::mem::transmute::<*mut c_void, GetIntegerv>(address) })
    }
}
