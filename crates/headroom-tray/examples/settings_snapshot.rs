use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use adw::prelude::*;
use anyhow::{Context, Result, anyhow};
use gtk::{glib, graphene};
use headroom_tray::i18n::Lang;
use headroom_tray::palette::{Palette, Scheme};
use headroom_tray::payload::parse_state;
use headroom_tray::preferences::model::{decode_settings, settings_from};
use headroom_tray::preferences::registry::parse_providers;
use headroom_tray::ui::prefs::{Service, SettingsWindow, Snapshot};
use headroom_tray::update::UpdateRun;

const USAGE: &str = "usage: settings_snapshot <state.json> <settings.json> <providers.json> \
                     <general|accounts|notifications|about|add|service> <out.png> [dark] [ru]";
const SETTLE: Duration = Duration::from_millis(2500);
const OPEN_DELAY: Duration = Duration::from_millis(300);
const PAGES: [&str; 4] = ["general", "accounts", "notifications", "about"];

struct Options {
    state: String,
    settings: String,
    providers: String,
    page: String,
    out: PathBuf,
    scheme: Scheme,
    lang: Lang,
}

fn options() -> Result<Options> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [state, settings, providers, page, out, rest @ ..] = args.as_slice() else {
        return Err(anyhow!(USAGE));
    };
    let has = |flag: &str| rest.iter().any(|arg| arg == flag);
    Ok(Options {
        state: state.clone(),
        settings: settings.clone(),
        providers: providers.clone(),
        page: page.clone(),
        out: PathBuf::from(out),
        scheme: if has("dark") {
            Scheme::Dark
        } else {
            Scheme::Light
        },
        lang: if has("ru") { Lang::Ru } else { Lang::En },
    })
}

fn read(path: &str) -> Result<String> {
    std::fs::read_to_string(path).with_context(|| format!("reading {path}"))
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
        .and_then(|native| native.renderer())
        .ok_or_else(|| anyhow!("no renderer"))?;
    #[allow(clippy::cast_precision_loss, reason = "window sizes are small")]
    let bounds = graphene::Rect::new(0.0, 0.0, width as f32, height as f32);
    renderer
        .render_texture(node, Some(&bounds))
        .save_to_png(out)?;
    Ok(())
}

fn show(options: &Options) -> Result<Rc<SettingsWindow>> {
    let state = parse_state(&read(&options.state)?)?;
    let settings = settings_from(&decode_settings(&read(&options.settings)?)?)?;
    let providers = parse_providers(&read(&options.providers)?);
    let palette = Palette::load(options.scheme)?;
    let window = SettingsWindow::new(None, Rc::new(|_| {}));
    let stopped = options.page == "service";
    let snapshot = Snapshot {
        lang: options.lang,
        logo_color: palette.css_value("text-secondary")?.to_owned(),
        service: if stopped {
            Service::Stopped {
                starting: false,
                error: None,
            }
        } else {
            Service::Running
        },
        settings: Some(&settings),
        state: Some(&state),
        providers: Some(&providers),
        update_run: &UpdateRun::Idle,
    };
    window.update(&snapshot);
    window.present();
    let page = options.page.split('/').next().unwrap_or_default();
    if let Some(index) = PAGES.iter().position(|known| *known == page)
        && !window.select_page(index)
    {
        return Err(anyhow!("page {} is not shown", options.page));
    }
    if options.page.starts_with("add") {
        let opener = Rc::clone(&window);
        glib::timeout_add_local_once(OPEN_DELAY, move || opener.open_add_dialog());
    }
    Ok(window)
}

fn detach_dialog(window: &SettingsWindow) -> Option<gtk::Window> {
    let dialog = window.window().visible_dialog()?;
    let content = dialog.child()?;
    dialog.set_child(None::<&gtk::Widget>);
    let (width, height) = (dialog.content_width(), dialog.content_height());
    dialog.force_close();
    let sheet = gtk::Window::builder()
        .decorated(false)
        .default_width(width)
        .default_height(height)
        .child(&content)
        .build();
    sheet.present();
    Some(sheet)
}

fn descendants(root: &gtk::Widget) -> Vec<gtk::Widget> {
    let mut found = vec![root.clone()];
    let mut child = root.first_child();
    while let Some(widget) = child {
        found.extend(descendants(&widget));
        child = widget.next_sibling();
    }
    found
}

fn press(root: &gtk::Widget, label: &str) {
    if let Some(text) = label.strip_prefix("type=") {
        let entry = descendants(root)
            .into_iter()
            .find_map(|widget| widget.downcast::<adw::PasswordEntryRow>().ok());
        if let Some(entry) = entry {
            entry.set_text(text);
        }
        return;
    }
    for widget in descendants(root).into_iter().filter(gtk::Widget::is_mapped) {
        if let Some(row) = widget.downcast_ref::<adw::ExpanderRow>()
            && row.title() == label
        {
            row.set_expanded(true);
            return;
        }
        if let Some(row) = widget.downcast_ref::<adw::ActionRow>()
            && row.title() == label
        {
            row.emit_activate();
            return;
        }
        if let Some(button) = widget.downcast_ref::<gtk::Button>()
            && button.label().as_deref() == Some(label)
        {
            button.emit_clicked();
            return;
        }
    }
    eprintln!("nothing labelled {label}");
}

fn walk(sheet: &gtk::Widget, steps: Vec<String>) {
    let mut delay = OPEN_DELAY;
    for step in steps {
        let target = sheet.clone();
        delay += OPEN_DELAY;
        glib::timeout_add_local_once(delay, move || press(&target, &step));
    }
}

fn main() -> Result<()> {
    let options = options()?;
    adw::init()?;
    if let Some(settings) = gtk::Settings::default() {
        settings.set_gtk_enable_animations(false);
    }
    adw::StyleManager::default().set_color_scheme(match options.scheme {
        Scheme::Dark => adw::ColorScheme::ForceDark,
        Scheme::Light => adw::ColorScheme::ForceLight,
    });
    let window = show(&options)?;
    let main_loop = glib::MainLoop::new(None, false);
    let result = Rc::new(RefCell::new(Ok(())));
    let detached: Rc<RefCell<Option<gtk::Window>>> = Rc::default();
    let (source, target) = (Rc::clone(&window), Rc::clone(&detached));
    let steps: Vec<String> = options.page.split('/').skip(1).map(str::to_owned).collect();
    glib::timeout_add_local_once(OPEN_DELAY * 2, move || {
        let sheet = detach_dialog(&source);
        match &sheet {
            Some(sheet) => walk(sheet.upcast_ref(), steps),
            None => walk(source.window().upcast_ref(), steps),
        }
        *target.borrow_mut() = sheet;
    });
    let (quit, saved, out) = (main_loop.clone(), Rc::clone(&result), options.out.clone());
    glib::timeout_add_local_once(SETTLE, move || {
        let shown: gtk::Widget = detached
            .borrow()
            .clone()
            .map_or_else(|| window.window().clone().upcast(), Cast::upcast);
        *saved.borrow_mut() = save(&shown, &out);
        quit.quit();
    });
    main_loop.run();
    result.replace(Ok(()))
}
