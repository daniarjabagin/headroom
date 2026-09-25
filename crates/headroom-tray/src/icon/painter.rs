use std::collections::HashMap;
use std::sync::Arc;

use super::Pixmap;
use super::glyph::Glyph;
use super::layout::{FULL_OPACITY, render};
use super::mark::parse_mark;
use super::shapes::Polygon;
use super::tints::Tints;
use crate::palette::Palette;
use crate::payload::State;

const CACHE_CAPACITY: usize = 48;

pub type Pixmaps = Arc<[Pixmap]>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayIcon {
    pub glyph: Glyph,
    pub tints: Option<Tints>,
    pub motion: bool,
}

impl TrayIcon {
    #[must_use]
    pub fn new(state: Option<&State>, palette: &Palette, motion: bool) -> Self {
        let tints = Tints::from_palette(palette)
            .inspect_err(|error| tracing::error!(%error, "the tray icon colors are unavailable"))
            .ok();
        Self {
            glyph: Glyph::from_state(state),
            tints,
            motion,
        }
    }

    #[must_use]
    pub fn themed() -> Self {
        Self {
            glyph: Glyph::Mark(None),
            tints: None,
            motion: false,
        }
    }

    #[must_use]
    pub fn pulses(&self) -> bool {
        self.motion && self.tints.is_some() && self.glyph.has_critical()
    }
}

type Key = (Glyph, Tints, u8);

#[derive(Debug, Default)]
pub struct PixmapCache {
    entries: HashMap<Key, Option<Pixmaps>>,
}

impl PixmapCache {
    pub fn get_or_render(
        &mut self,
        key: Key,
        render: impl FnOnce() -> Option<Vec<Pixmap>>,
    ) -> Option<Pixmaps> {
        if let Some(found) = self.entries.get(&key) {
            return found.clone();
        }
        if self.entries.len() >= CACHE_CAPACITY {
            self.entries.clear();
        }
        let rendered: Option<Pixmaps> = render().map(Into::into);
        self.entries.insert(key, rendered.clone());
        rendered
    }
}

pub struct Painter {
    cache: PixmapCache,
    plates: Vec<Polygon>,
}

impl Default for Painter {
    fn default() -> Self {
        let plates = parse_mark(crate::assets::MARK)
            .inspect_err(|error| tracing::error!(%error, "the Headroom mark cannot be tinted"))
            .unwrap_or_default();
        Self {
            cache: PixmapCache::default(),
            plates,
        }
    }
}

impl Painter {
    pub fn paint(&mut self, icon: &TrayIcon, opacity: u8) -> Option<Pixmaps> {
        let tints = icon.tints?;
        let tinted_mark = matches!(icon.glyph, Glyph::Mark(_));
        if icon.glyph.is_themed_mark() || (tinted_mark && self.plates.is_empty()) {
            return None;
        }
        let opacity = if icon.glyph.has_critical() {
            opacity
        } else {
            FULL_OPACITY
        };
        let plates = &self.plates;
        self.cache
            .get_or_render((icon.glyph.clone(), tints, opacity), || {
                render(&icon.glyph, plates, &tints, opacity)
            })
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;
    use crate::icon::glyph::Gauge;
    use crate::palette::Palettes;
    use crate::payload::Tone;

    fn icon(percent: u8, tone: Tone) -> TrayIcon {
        let palettes = Palettes::load().unwrap();
        TrayIcon {
            glyph: Glyph::Rings(vec![Gauge {
                percent,
                tick: None,
                tone,
            }]),
            tints: Tints::from_palette(&palettes.dark).ok(),
            motion: true,
        }
    }

    impl PixmapCache {
        fn len(&self) -> usize {
            self.entries.len()
        }
    }

    #[test]
    fn cache_hits_skip_rendering() {
        let mut cache = PixmapCache::default();
        let renders = Cell::new(0);
        let key = icon(40, Tone::Good);
        let key = (key.glyph, key.tints.unwrap(), 255);
        let render = || {
            renders.set(renders.get() + 1);
            Some(vec![Pixmap {
                size: 1,
                argb: vec![0; 4],
            }])
        };
        let first = cache.get_or_render(key.clone(), render).unwrap();
        let second = cache.get_or_render(key.clone(), render).unwrap();
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(renders.get(), 1);
        let mut other = key;
        other.2 = 140;
        cache.get_or_render(other, render);
        assert_eq!((renders.get(), cache.len()), (2, 2));
    }

    #[test]
    fn cache_stays_bounded() {
        let mut cache = PixmapCache::default();
        let base = icon(0, Tone::Good);
        for percent in 0..=100u8 {
            let key = (icon(percent, Tone::Good).glyph, base.tints.unwrap(), 255);
            cache.get_or_render(key, || None);
        }
        assert!(cache.len() <= CACHE_CAPACITY);
        assert!(cache.len() > 0);
    }

    #[test]
    fn painter_reuses_frames_and_ignores_opacity_without_critical() {
        let mut painter = Painter::default();
        let good = icon(40, Tone::Good);
        let full = painter.paint(&good, 255).unwrap();
        let dim = painter.paint(&good, 140).unwrap();
        assert!(Arc::ptr_eq(&full, &dim));
        let critical = icon(5, Tone::Critical);
        let a = painter.paint(&critical, 255).unwrap();
        let b = painter.paint(&critical, 140).unwrap();
        assert!(!Arc::ptr_eq(&a, &b));
        assert!(painter.paint(&TrayIcon::themed(), 255).is_none());
    }

    #[test]
    fn only_critical_icons_with_motion_pulse() {
        assert!(icon(5, Tone::Critical).pulses());
        assert!(!icon(5, Tone::Warning).pulses());
        let still = TrayIcon {
            motion: false,
            ..icon(5, Tone::Critical)
        };
        assert!(!still.pulses());
    }
}
