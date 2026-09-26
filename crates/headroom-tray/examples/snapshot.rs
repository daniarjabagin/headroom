use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

use adw::prelude::*;
use anyhow::{Context, Result, anyhow};
use gtk::{gdk, glib, graphene};
use headroom_tray::dates::{Clock, Locale};
use headroom_tray::i18n::Lang;
use headroom_tray::palette::{Palette, Scheme};
use headroom_tray::payload::{State, parse_state};
use headroom_tray::popup_model::share::{ShareCard, ShareInput, account_card, group_card};
use headroom_tray::popup_model::spend_view::{SpendChoice, SpendOverride};
use headroom_tray::preferences::registry::parse_providers;
use headroom_tray::ui::context::{Ctx, UiState};
use headroom_tray::ui::popup::Frame;
use headroom_tray::ui::popup_tree::PopupTree;
use headroom_tray::ui::preview::popover_parts;
use headroom_tray::ui::share::render_card;
use headroom_tray::ui::style::Styles;
use headroom_tray::ui::toast::ToastMessage;
use headroom_tray::view::View;
use jiff::Timestamp;
use jiff::tz::TimeZone;

const USAGE: &str = "usage: snapshot <state.json|loading|unavailable|failed> <out.png> \
                     [dark] [ru] [12h] [expanded] [parts] [share] [toast] [sheen] [hover] [providers=<providers.json>]";
const SETTLE: Duration = Duration::from_millis(600);
const MID_SHEEN: Duration = Duration::from_millis(1700);

#[allow(
    clippy::struct_excessive_bools,
    reason = "each flag is an independent command-line switch"
)]
struct Options {
    source: String,
    out: PathBuf,
    scheme: Scheme,
    lang: Lang,
    clock: Clock,
    expanded: bool,
    parts: bool,
    share: bool,
    toast: bool,
    sheen: bool,
    hover: bool,
    providers: Option<String>,
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
        clock: if has("12h") { Clock::H12 } else { Clock::H24 },
        expanded: has("expanded"),
        parts: has("parts"),
        share: has("share"),
        toast: has("toast"),
        sheen: has("sheen"),
        hover: has("hover"),
        providers: rest
            .iter()
            .find_map(|arg| arg.strip_prefix("providers="))
            .map(str::to_owned),
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

fn links(
    options: &Options,
) -> Result<Rc<BTreeMap<String, headroom_tray::preferences::registry::ProviderLinks>>> {
    let Some(path) = &options.providers else {
        return Ok(Rc::default());
    };
    let json = std::fs::read_to_string(path).with_context(|| format!("reading {path}"))?;
    let providers = parse_providers(&json)?;
    Ok(Rc::new(
        providers
            .into_iter()
            .map(|provider| (provider.id, provider.links))
            .collect(),
    ))
}

fn context(options: &Options, palette: Palette, view: &View) -> Result<Ctx> {
    let display = view
        .state()
        .map(|state| state.display.clone())
        .unwrap_or_default();
    let recent = view.state().is_some_and(State::speaks_0_6);
    let spend = view.state().map(|state| {
        SpendChoice::resolve(&display, SpendOverride::default(), &state.spend, recent)
    });
    Ok(Ctx {
        locale: Locale::new(options.lang, TimeZone::UTC).with_clock(options.clock),
        display,
        palette,
        motion: options.sheen,
        offline: view.state().is_some_and(|state| state.offline),
        sign_in: BTreeSet::from(["claude".to_owned(), "codex".to_owned()]),
        ui: UiState {
            more_expanded: options.expanded,
            ..UiState::default()
        },
        version: format!("Headroom {}", env!("CARGO_PKG_VERSION")),
        act: Rc::new(|_| {}),
        ticks: RefCell::default(),
        links: links(options)?,
        spend,
        recent,
        textures: Rc::default(),
    })
}

fn save(widget: &gtk::Widget, out: &Path) -> Result<()> {
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

fn sibling(out: &Path, suffix: &str) -> PathBuf {
    let stem = out
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("snapshot");
    out.with_file_name(format!("{stem}-{suffix}.png"))
}

fn show(child: &gtk::Widget, width: i32) -> gtk::Window {
    let window = gtk::Window::builder()
        .decorated(false)
        .default_width(width)
        .css_classes(["headroom-window"])
        .child(child)
        .build();
    window.present();
    window
}

fn share_cards(state: &State, locale: &Locale) -> Vec<ShareCard> {
    let input = ShareInput {
        locale,
        display: &state.display,
        headline: state.headline.as_ref(),
        now: state.generated_at,
    };
    let groups = state
        .combined
        .iter()
        .filter_map(|group| group_card(input, group, &state.accounts));
    let singles = state
        .accounts
        .iter()
        .filter(|account| {
            !state
                .combined
                .iter()
                .any(|g| g.account_ids.contains(&account.id))
        })
        .filter_map(|account| account_card(input, account));
    groups.chain(singles).collect()
}

fn save_shares(root: &gtk::Widget, ctx: &Ctx, state: &State, out: &Path) -> Result<()> {
    for card in share_cards(state, &ctx.locale) {
        let texture = render_card(root, &card, &ctx.palette, ctx.locale.lang)?;
        texture.save_to_png(sibling(out, &format!("share-{}", card.provider)))?;
    }
    Ok(())
}

fn install_styles(scheme: Scheme) -> Result<Palette> {
    let display = gdk::Display::default().ok_or_else(|| anyhow!("no display"))?;
    let palette = Palette::load(scheme)?;
    let mut styles = Styles::install(&display);
    styles.apply(&palette)?;
    adw::StyleManager::default().set_color_scheme(match scheme {
        Scheme::Dark => adw::ColorScheme::ForceDark,
        Scheme::Light => adw::ColorScheme::ForceLight,
    });
    Ok(palette)
}

fn show_toast(tree: &PopupTree, lang: Lang) {
    let message = ToastMessage {
        ok: true,
        title: lang.tr("Image copied").to_owned(),
        detail: Some(
            lang.tr("saved to {folder}")
                .replace("{folder}", "Pictures/Headroom"),
        ),
    };
    tree.toast(&message, false);
}

type Part = (PathBuf, gtk::Widget, gtk::Window);

fn show_parts(ctx: &Ctx, view: &View, out: &Path) -> Vec<Part> {
    let Some(state) = view.state() else {
        return Vec::new();
    };
    popover_parts(ctx, state)
        .into_iter()
        .map(|part| {
            let frame = gtk::Box::new(gtk::Orientation::Vertical, 0);
            frame.add_css_class("headroom-popup");
            frame.add_css_class("headroom-preview-backdrop");
            frame.append(&part.widget);
            let window = show(frame.upcast_ref(), -1);
            (sibling(out, part.name), frame.upcast(), window)
        })
        .collect()
}

fn hover_everything(widget: &gtk::Widget) {
    widget.set_state_flags(gtk::StateFlags::PRELIGHT, false);
    if widget.has_css_class("headroom-section-header") {
        widget.add_css_class("hovered");
    }
    let mut child = widget.first_child();
    while let Some(current) = child {
        hover_everything(&current);
        child = current.next_sibling();
    }
}

fn main() -> Result<()> {
    let options = options()?;
    adw::init()?;
    let (view, now) = view(&options.source)?;
    let palette = install_styles(options.scheme)?;
    let frame = Frame {
        now,
        max_height: 4000,
        animate: false,
    };
    let ctx = context(&options, palette, &view)?;
    let mut tree = PopupTree::default();
    let root = tree.mount(&ctx, &view, &frame);
    if options.toast {
        show_toast(&tree, options.lang);
    }
    let _popup = show(root.upcast_ref(), 320);
    let parts = if options.parts {
        show_parts(&ctx, &view, &options.out)
    } else {
        Vec::new()
    };
    let main_loop = glib::MainLoop::new(None, false);
    let (quit, out, result) = (
        main_loop.clone(),
        options.out.clone(),
        Rc::new(RefCell::new(Ok(()))),
    );
    let saved = Rc::clone(&result);
    let hover = options.hover.then(|| root.clone());
    let share = options.share.then(|| view.state().cloned()).flatten();
    let settle = if options.sheen { MID_SHEEN } else { SETTLE };
    glib::timeout_add_local_once(SETTLE / 2, move || {
        if let Some(root) = &hover {
            eprintln!("height before hover: {}", root.height());
            hover_everything(root.upcast_ref());
        }
    });
    glib::timeout_add_local_once(settle, move || {
        if options.hover {
            eprintln!("height after hover: {}", root.height());
        }
        let mut outcome = save(root.upcast_ref(), &out);
        for (path, widget, _window) in &parts {
            outcome = outcome.and_then(|()| save(widget, path));
        }
        if let Some(state) = &share {
            outcome = outcome.and_then(|()| save_shares(root.upcast_ref(), &ctx, state, &out));
        }
        *saved.borrow_mut() = outcome;
        quit.quit();
    });
    main_loop.run();
    result.replace(Ok(()))
}
