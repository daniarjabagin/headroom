use std::path::PathBuf;

use anyhow::{Result, anyhow};
use gtk::{gdk, glib};
use headroom_tray::icon::{FULL_OPACITY, Gauge, Glyph, Painter, Pixmap, SIZES, Tints, TrayIcon};
use headroom_tray::palette::{Palette, Scheme};
use headroom_tray::payload::Tone;

const ZOOM: usize = 6;
const CELL: usize = 70 * ZOOM;

fn gauge(percent: u8, tick: Option<u8>, tone: Tone) -> Gauge {
    Gauge {
        percent,
        tick,
        tone,
    }
}

fn variants() -> Vec<Glyph> {
    let good = gauge(72, Some(59), Tone::Good);
    let warning = gauge(40, Some(48), Tone::Warning);
    let critical = gauge(8, Some(10), Tone::Critical);
    vec![
        Glyph::Rings(vec![good]),
        Glyph::Rings(vec![warning]),
        Glyph::Rings(vec![critical]),
        Glyph::Rings(vec![good, warning]),
        Glyph::Bars(vec![good]),
        Glyph::Bars(vec![warning]),
        Glyph::Bars(vec![good, critical]),
        Glyph::Mark(Some(Tone::Warning)),
        Glyph::Mark(Some(Tone::Critical)),
    ]
}

fn blend(background: u8, pixmap: &Pixmap, x: usize, y: usize) -> Result<[u8; 3]> {
    let side = usize::try_from(pixmap.size)?;
    let offset = (y * side + x) * 4;
    let alpha = u32::from(pixmap.argb[offset]);
    let mix = |channel: u8| {
        let value = (u32::from(channel) * alpha + u32::from(background) * (255 - alpha)) / 255;
        u8::try_from(value).unwrap_or(u8::MAX)
    };
    Ok([
        mix(pixmap.argb[offset + 1]),
        mix(pixmap.argb[offset + 2]),
        mix(pixmap.argb[offset + 3]),
    ])
}

struct Sheet {
    width: usize,
    background: u8,
    pixels: Vec<u8>,
}

impl Sheet {
    fn place(&mut self, column: usize, row: usize, pixmap: &Pixmap) -> Result<()> {
        let side = usize::try_from(pixmap.size)? * ZOOM;
        let (left, top) = (column * CELL + ZOOM, row * CELL + ZOOM);
        for y in 0..side {
            for x in 0..side {
                let rgb = blend(self.background, pixmap, x / ZOOM, y / ZOOM)?;
                let offset = ((top + y) * self.width + left + x) * 3;
                self.pixels[offset..offset + 3].copy_from_slice(&rgb);
            }
        }
        Ok(())
    }

    fn save(self, path: PathBuf) -> Result<()> {
        let height = self.pixels.len() / (self.width * 3);
        let texture = gdk::MemoryTexture::new(
            i32::try_from(self.width)?,
            i32::try_from(height)?,
            gdk::MemoryFormat::R8g8b8,
            &glib::Bytes::from_owned(self.pixels),
            self.width * 3,
        );
        gtk::prelude::TextureExt::save_to_png(&texture, path)?;
        Ok(())
    }
}

fn sheet(scheme: Scheme, background: u8) -> Result<Sheet> {
    let tints = Tints::from_palette(&Palette::load(scheme)?)?;
    let glyphs = variants();
    let width = CELL * SIZES.len();
    let mut sheet = Sheet {
        width,
        background,
        pixels: vec![background; width * CELL * glyphs.len() * 3],
    };
    let mut painter = Painter::default();
    for (row, glyph) in glyphs.into_iter().enumerate() {
        let icon = TrayIcon {
            glyph,
            tints: Some(tints),
            motion: false,
        };
        let pixmaps = painter
            .paint(&icon, FULL_OPACITY)
            .ok_or_else(|| anyhow!("the glyph was not drawn"))?;
        for (column, pixmap) in pixmaps.iter().enumerate() {
            sheet.place(column, row, pixmap)?;
        }
    }
    Ok(sheet)
}

fn main() -> Result<()> {
    let out = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or_else(|| anyhow!("usage: tray_icons <dir>"))?,
    );
    sheet(Scheme::Light, 0xf0)?.save(out.join("tray-light.png"))?;
    sheet(Scheme::Dark, 0x24)?.save(out.join("tray-dark.png"))?;
    Ok(())
}
