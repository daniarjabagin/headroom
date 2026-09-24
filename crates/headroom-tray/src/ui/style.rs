use gtk::gdk;

use crate::css::stylesheet;
use crate::palette::{Palette, PaletteError, Scheme};
use crate::payload::Theme;
use crate::theme::{SystemHints, resolve_scheme};

pub struct Styles {
    provider: gtk::CssProvider,
    loaded: Option<Scheme>,
}

impl Styles {
    #[must_use]
    pub fn install(display: &gdk::Display) -> Self {
        let provider = gtk::CssProvider::new();
        provider.connect_parsing_error(|_, section, error| {
            tracing::warn!(%error, location = %section, "stylesheet error");
        });
        gtk::style_context_add_provider_for_display(
            display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        Self {
            provider,
            loaded: None,
        }
    }

    pub fn apply(&mut self, palette: &Palette) -> Result<(), PaletteError> {
        if self.loaded != Some(palette.scheme()) {
            self.provider.load_from_string(&stylesheet(palette)?);
            self.loaded = Some(palette.scheme());
        }
        Ok(())
    }
}

fn follows_system(theme: Theme, manager: &adw::StyleManager) -> bool {
    theme == Theme::System && manager.system_supports_color_schemes()
}

#[must_use]
pub fn resolve_theme(theme: Theme) -> Scheme {
    let manager = adw::StyleManager::default();
    let system = follows_system(theme, &manager);
    if system {
        manager.set_color_scheme(adw::ColorScheme::Default);
    }
    let hints = SystemHints {
        supports_color_schemes: manager.system_supports_color_schemes(),
        system_dark: manager.is_dark(),
        gtk_theme: gtk::Settings::default()
            .and_then(|s| s.gtk_theme_name())
            .map(Into::into),
    };
    let scheme = resolve_scheme(theme, &hints);
    if !system {
        manager.set_color_scheme(match scheme {
            Scheme::Dark => adw::ColorScheme::ForceDark,
            Scheme::Light => adw::ColorScheme::ForceLight,
        });
    }
    scheme
}
