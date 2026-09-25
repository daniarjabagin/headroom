use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use adw::prelude::*;
use anyhow::{Context, Result, anyhow};
use gtk::{glib, graphene};
use headroom_tray::i18n::Lang;
use headroom_tray::palette::{Palette, Scheme};
use headroom_tray::payload::{State, parse_state};
use headroom_tray::preferences::model::{Settings, decode_settings, settings_from};
use headroom_tray::preferences::registry::{ProviderInfo, RegistryError, parse_providers};
use headroom_tray::ui::onboarding::{OnboardingSetup, OnboardingWindow};
use headroom_tray::ui::prefs::{Act, Service, SettingsWindow, Snapshot, keyboard};
use headroom_tray::update::UpdateRun;
use headroom_tray::update_check::CheckRun;

const USAGE: &str = "usage: settings_snapshot <state.json> <settings.json> <providers.json> \
                     <general|accounts|notifications|advanced|about|add|service|onboarding>[/step…] \
                     <out.png> [dark] [ru]; steps: a label, scroll=<group>, type=<text>, toast=<text>, capture=<accelerator>";
const SETTLE: Duration = Duration::from_millis(4000);
const OPEN_DELAY: Duration = Duration::from_millis(300);
const PAGES: [&str; 5] = ["general", "accounts", "notifications", "advanced", "about"];
const LOG_FILE: &str = "~/.local/state/headroom/headroom.log";

struct Options {
    state: String,
    settings: String,
    providers: String,
    page: String,
    out: PathBuf,
    scheme: Scheme,
    lang: Lang,
}

struct Inputs {
    state: State,
    settings: Settings,
    providers: Result<Vec<ProviderInfo>, RegistryError>,
    logo_color: String,
}

enum Shown {
    Settings(Rc<SettingsWindow>),
    Onboarding(Rc<OnboardingWindow>),
}

impl Shown {
    fn window(&self) -> gtk::Window {
        match self {
            Shown::Settings(window) => window.window().clone().upcast(),
            Shown::Onboarding(window) => window.window().clone().upcast(),
        }
    }
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

fn inputs(options: &Options) -> Result<Inputs> {
    let palette = Palette::load(options.scheme)?;
    Ok(Inputs {
        state: parse_state(&read(&options.state)?)?,
        settings: settings_from(&decode_settings(&read(&options.settings)?)?)?,
        providers: parse_providers(&read(&options.providers)?),
        logo_color: palette.css_value("text-secondary")?.to_owned(),
    })
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

fn show_onboarding(options: &Options, inputs: &Inputs) -> Rc<OnboardingWindow> {
    let setup = OnboardingSetup {
        lang: options.lang,
        logo_color: inputs.logo_color.clone(),
        act: Rc::new(|_| {}),
        shortcut_supported: true,
        on_closed: Rc::new(|| {}),
    };
    let window = OnboardingWindow::new(None, &setup);
    window.update(&inputs.state, &inputs.settings, Some(&inputs.providers));
    if options.page.contains("/done") {
        window.show_done();
    }
    window.present();
    window
}

fn show_settings(options: &Options, inputs: &Inputs) -> Result<Rc<SettingsWindow>> {
    let window = SettingsWindow::new(None, Rc::new(|_| {}));
    let stopped = options.page == "service";
    let snapshot = Snapshot {
        lang: options.lang,
        logo_color: inputs.logo_color.clone(),
        service: if stopped {
            Service::Stopped {
                starting: false,
                error: None,
            }
        } else {
            Service::Running
        },
        settings: Some(&inputs.settings),
        state: Some(&inputs.state),
        providers: Some(&inputs.providers),
        update_run: &UpdateRun::Idle,
        update_check: &CheckRun::Idle,
        shortcut_supported: true,
    };
    window.update(&snapshot);
    window.diagnostics(
        options.lang,
        Ok(headroom_tray::ui::prefs::diagnostics::Diagnostics {
            text: String::new(),
            log_file: Some(LOG_FILE.to_owned()),
        }),
    );
    window.present();
    let page = options.page.split('/').next().unwrap_or_default();
    if let Some(index) = PAGES.iter().position(|known| *known == page)
        && !window.select_page(index)
    {
        return Err(anyhow!("page {} is not shown", options.page));
    }
    if options.page.starts_with("add") {
        let opener = Rc::clone(&window);
        glib::timeout_add_local_once(OPEN_DELAY, move || opener.open_add_dialog(None));
    }
    Ok(window)
}

fn show(options: &Options) -> Result<Shown> {
    let inputs = inputs(options)?;
    if options.page.starts_with("onboarding") {
        return Ok(Shown::Onboarding(show_onboarding(options, &inputs)));
    }
    Ok(Shown::Settings(show_settings(options, &inputs)?))
}

fn detach_dialog(shown: &Shown) -> Option<gtk::Window> {
    let Shown::Settings(window) = shown else {
        return None;
    };
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

fn type_text(root: &gtk::Widget, text: &str) {
    let entry = descendants(root)
        .into_iter()
        .find_map(|widget| widget.downcast::<adw::PasswordEntryRow>().ok());
    if let Some(entry) = entry {
        entry.set_text(text);
    }
}

fn press_widget(widget: &gtk::Widget, label: &str) -> bool {
    if let Some(row) = widget.downcast_ref::<adw::ExpanderRow>()
        && row.title() == label
    {
        row.set_expanded(true);
        return true;
    }
    if let Some(row) = widget.downcast_ref::<adw::ActionRow>()
        && row.title() == label
    {
        row.emit_activate();
        return true;
    }
    if let Some(button) = widget.downcast_ref::<gtk::Button>()
        && button.label().as_deref() == Some(label)
    {
        button.emit_clicked();
        return true;
    }
    if let Some(row) = widget.downcast_ref::<gtk::ListBoxRow>()
        && row
            .child()
            .and_downcast::<gtk::Label>()
            .is_some_and(|text| text.label() == label)
    {
        row.emit_activate();
        return true;
    }
    false
}

fn scroll_to(root: &gtk::Widget, title: &str) {
    let group = descendants(root)
        .into_iter()
        .filter(gtk::Widget::is_mapped)
        .filter_map(|widget| widget.downcast::<adw::PreferencesGroup>().ok())
        .find(|group| group.title() == title);
    let Some(group) = group else {
        eprintln!("no group titled {title}");
        return;
    };
    let Some(scroller) = group
        .ancestor(gtk::ScrolledWindow::static_type())
        .and_downcast::<gtk::ScrolledWindow>()
    else {
        return;
    };
    let Some(content) = scroller.child() else {
        return;
    };
    if let Some(point) = group.compute_point(&content, &graphene::Point::zero()) {
        scroller.vadjustment().set_value(f64::from(point.y()));
    }
}

fn press(root: &gtk::Widget, label: &str) {
    if let Some(text) = label.strip_prefix("type=") {
        type_text(root, text);
        return;
    }
    if let Some(title) = label.strip_prefix("scroll=") {
        scroll_to(root, title);
        return;
    }
    if let Some(accelerator) = label.strip_prefix("capture=") {
        let act: Act = Rc::new(|_| {});
        keyboard::open_capture(Lang::En, root, &act, accelerator);
        return;
    }
    let pressed = descendants(root)
        .into_iter()
        .filter(gtk::Widget::is_mapped)
        .any(|widget| press_widget(&widget, label));
    if !pressed {
        eprintln!("nothing labelled {label}");
    }
}

fn walk(sheet: &gtk::Widget, settings: Option<&Rc<SettingsWindow>>, steps: Vec<String>) {
    let mut delay = OPEN_DELAY;
    for step in steps {
        let (target, settings) = (sheet.clone(), settings.cloned());
        delay += OPEN_DELAY;
        glib::timeout_add_local_once(delay, move || match step.strip_prefix("toast=") {
            Some(text) => {
                if let Some(settings) = settings {
                    settings.toast(text);
                }
            }
            None => press(&target, &step),
        });
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
    let shown = Rc::new(show(&options)?);
    let main_loop = glib::MainLoop::new(None, false);
    let result = Rc::new(RefCell::new(Ok(())));
    let detached: Rc<RefCell<Option<gtk::Window>>> = Rc::default();
    let (source, target) = (Rc::clone(&shown), Rc::clone(&detached));
    let steps: Vec<String> = options
        .page
        .split('/')
        .skip(1)
        .filter(|step| *step != "done")
        .map(str::to_owned)
        .collect();
    glib::timeout_add_local_once(OPEN_DELAY * 2, move || {
        let sheet = detach_dialog(&source);
        let settings = match source.as_ref() {
            Shown::Settings(window) => Some(Rc::clone(window)),
            Shown::Onboarding(_) => None,
        };
        match &sheet {
            Some(sheet) => walk(sheet.upcast_ref(), settings.as_ref(), steps),
            None => walk(source.window().upcast_ref(), settings.as_ref(), steps),
        }
        *target.borrow_mut() = sheet;
    });
    let (quit, saved, out) = (main_loop.clone(), Rc::clone(&result), options.out.clone());
    glib::timeout_add_local_once(SETTLE, move || {
        let window: gtk::Widget = detached
            .borrow()
            .clone()
            .map_or_else(|| shown.window().upcast(), Cast::upcast);
        *saved.borrow_mut() = save(&window, &out);
        quit.quit();
    });
    main_loop.run();
    result.replace(Ok(()))
}
