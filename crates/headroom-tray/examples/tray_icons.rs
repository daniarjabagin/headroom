use std::path::PathBuf;

use anyhow::{Result, anyhow};
use gtk::{gdk, glib};
use headroom_tray::icon::{RingKey, ring_pixmaps};
use headroom_tray::palette::{Palette, Scheme};
use headroom_tray::payload::Tone;

const KEYS: [(u8, Tone); 4] = [
    (73, Tone::Good),
    (32, Tone::Warning),
    (8, Tone::Critical),
    (100, Tone::Neutral),
];

fn main() -> Result<()> {
    let out = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or_else(|| anyhow!("usage: tray_icons <dir>"))?,
    );
    let palette = Palette::load(Scheme::Dark)?;
    for (percent, tone) in KEYS {
        let key = RingKey { percent, tone };
        for pixmap in ring_pixmaps(key, palette.tone(tone)?) {
            let side = i32::try_from(pixmap.size)?;
            let stride = usize::try_from(side)? * 4;
            let texture = gdk::MemoryTexture::new(
                side,
                side,
                gdk::MemoryFormat::A8r8g8b8,
                &glib::Bytes::from_owned(pixmap.argb),
                stride,
            );
            let name = format!("ring-{percent}-{tone:?}-{side}.png").to_lowercase();
            gtk::prelude::TextureExt::save_to_png(&texture, out.join(name))?;
        }
    }
    Ok(())
}
