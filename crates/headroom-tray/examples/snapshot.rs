use std::cell::RefCell;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use adw::prelude::*;
use anyhow::{Context, Result, anyhow};
use gtk::{gdk, glib, graphene};
use headroom_tray::dates::Locale;
use headroom_tray::i18n::Lang;
use headroom_tray::palette::{Palette, Scheme};
use headroom_tray::payload::parse_state;
use headroom_tray::ui::build;
use headroom_tray::ui::context::{Ctx, UiState};
use headroom_tray::ui::popup::Frame;
use headroom_tray::ui::style::Styles;
use headroom_tray::view::View;
use jiff::Timestamp;
use jiff::tz::TimeZone;

const USAGE: &str = "usage: snapshot <state.json|loading|unavailable|failed> <out.png> [dark] [ru]";
const SETTLE: Duration = Duration::from_millis(600);

struct Options {
    source: String,
    out: PathBuf,
    scheme: Scheme,
    lang: Lang,
}

fn options() -> Result<Options> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [source, out, rest @ ..] = args.as_slice() else {
        return Err(anyhow!(USAGE));
    };
    let has = |flag: &str| rest.iter().any(|arg| arg == flag);
    Ok(Options {
        source: source.clone(),
        out: PathBuf::from(out),
        scheme: if has("dark") {
            Scheme::Dark
        } else {
            Scheme::Light
        },
        lang: if has("ru") { Lang::Ru } else { Lang::En },
    })
}

fn view(source: &str) -> Result<(View, Timestamp)> {
    let now = Timestamp::now();
    Ok(match source {
        "loading" => (View::Loading, now),
        "unavailable" => (
            View::Unavailable {
                starting: false,
                error: None,
            },
            now,
        ),
        "failed" => (
            View::Failed("unreadable state from the Headroom service".into()),
            now,
        ),
        path => {
            let json = std::fs::read_to_string(path).with_context(|| format!("reading {path}"))?;
            let state = parse_state(&json)?;
            let now = state.generated_at;
            (View::Ready(Box::new(state)), now)
        }
    })
}

fn context(options: &Options, palette: Palette, view: &View) -> Ctx {
    Ctx {
        locale: Locale::new(options.lang, TimeZone::UTC),
        display: view
            .state()
            .map(|state| state.display.clone())
            .unwrap_or_default(),
        palette,
        motion: false,
        offline: view.state().is_some_and(|state| state.offline),
        sign_in: BTreeSet::from(["claude".to_owned(), "codex".to_owned()]),
        ui: UiState::default(),
        version: format!("Headroom {}", env!("CARGO_PKG_VERSION")),
        act: Rc::new(|_| {}),
        ticks: RefCell::default(),
    }
}

fn save(widget: &gtk::Widget, out: &PathBuf) -> Result<()> {
    let (width, height) = (widget.width(), widget.height());
    let paintable = gtk::WidgetPaintable::new(Some(widget));
    let snapshot = gtk::Snapshot::new();
    paintable.snapshot(&snapshot, f64::from(width), f64::from(height));
    let node = snapshot
        .to_node()
        .ok_or_else(|| anyhow!("nothing was drawn"))?;
    let renderer = widget
        .native()
        .ok_or_else(|| anyhow!("no native"))?
        .renderer();
    let renderer = renderer.ok_or_else(|| anyhow!("no renderer"))?;
    #[allow(clippy::cast_precision_loss, reason = "window sizes are small")]
    let bounds = graphene::Rect::new(0.0, 0.0, width as f32, height as f32);
    renderer
        .render_texture(node, Some(&bounds))
        .save_to_png(out)?;
    Ok(())
}

fn main() -> Result<()> {
    let options = options()?;
    adw::init()?;
    let display = gdk::Display::default().ok_or_else(|| anyhow!("no display"))?;
    let (view, now) = view(&options.source)?;
    let palette = Palette::load(options.scheme)?;
    let mut styles = Styles::install(&display);
    styles.apply(&palette)?;
    adw::StyleManager::default().set_color_scheme(match options.scheme {
        Scheme::Dark => adw::ColorScheme::ForceDark,
        Scheme::Light => adw::ColorScheme::ForceLight,
    });
    let ctx = context(&options, palette, &view);
    let frame = Frame {
        now,
        max_height: 4000,
        animate: false,
    };
    let (root, _ticks) = build(ctx, &view, &frame);
    let window = gtk::Window::builder()
        .decorated(false)
        .default_width(320)
        .css_classes(["headroom-window"])
        .child(&root)
        .build();
    window.present();
    let main_loop = glib::MainLoop::new(None, false);
    let (quit, out, result) = (
        main_loop.clone(),
        options.out.clone(),
        Rc::new(RefCell::new(Ok(()))),
    );
    let saved = Rc::clone(&result);
    glib::timeout_add_local_once(SETTLE, move || {
        *saved.borrow_mut() = save(root.upcast_ref(), &out);
        quit.quit();
    });
    main_loop.run();
    result.replace(Ok(()))
}
