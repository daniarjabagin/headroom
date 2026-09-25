use std::cell::RefCell;
use std::collections::HashMap;

use gtk::prelude::*;
use gtk::{gdk, glib};

use crate::assets::tinted_svg;

const SVG_RENDER_SCALE: i32 = 3;
const TEXTURE_CAPACITY: usize = 64;

pub fn label(text: &str, classes: &[&str]) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.set_xalign(0.0);
    for class in classes {
        label.add_css_class(class);
    }
    label
}

pub fn wrapping_label(text: &str, classes: &[&str]) -> gtk::Label {
    let label = label(text, classes);
    label.set_wrap(true);
    label.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    label.set_natural_wrap_mode(gtk::NaturalWrapMode::Word);
    label
}

pub fn row(spacing: i32, classes: &[&str]) -> gtk::Box {
    boxed(gtk::Orientation::Horizontal, spacing, classes)
}

pub fn column(spacing: i32, classes: &[&str]) -> gtk::Box {
    boxed(gtk::Orientation::Vertical, spacing, classes)
}

fn boxed(orientation: gtk::Orientation, spacing: i32, classes: &[&str]) -> gtk::Box {
    let container = gtk::Box::new(orientation, spacing);
    for class in classes {
        container.add_css_class(class);
    }
    container
}

pub fn spacer() -> gtk::Box {
    let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    spacer
}

pub fn button(
    child: &impl IsA<gtk::Widget>,
    classes: &[&str],
    on_click: impl Fn() + 'static,
) -> gtk::Button {
    let button = gtk::Button::new();
    button.set_child(Some(child));
    button.set_focus_on_click(false);
    for class in classes {
        button.add_css_class(class);
    }
    button.connect_clicked(move |_| on_click());
    button
}

pub fn text_button(text: &str, classes: &[&str], on_click: impl Fn() + 'static) -> gtk::Button {
    button(&label(text, &[]), classes, on_click)
}

pub fn icon(name: &str, size: i32, classes: &[&str]) -> gtk::Image {
    let image = gtk::Image::from_icon_name(name);
    image.set_pixel_size(size);
    for class in classes {
        image.add_css_class(class);
    }
    image
}

pub fn svg_texture(svg: &str, color: &str, size: i32) -> Option<gdk::Texture> {
    svg_texture_at(svg, color, u32::try_from(size * SVG_RENDER_SCALE).ok()?)
}

pub fn svg_texture_at(svg: &str, color: &str, pixels: u32) -> Option<gdk::Texture> {
    let tinted = tinted_svg(svg, color, pixels);
    let bytes = glib::Bytes::from_owned(tinted.into_bytes());
    match gdk::Texture::from_bytes(&bytes) {
        Ok(texture) => Some(texture),
        Err(error) => {
            tracing::warn!(%error, "could not load an embedded icon");
            None
        }
    }
}

fn picture(texture: Option<&gdk::Texture>, size: i32, classes: &[&str]) -> gtk::Image {
    let image = gtk::Image::new();
    if let Some(texture) = texture {
        image.set_paintable(Some(texture));
    }
    image.set_pixel_size(size);
    for class in classes {
        image.add_css_class(class);
    }
    image
}

pub fn svg_image(svg: &str, color: &str, size: i32, classes: &[&str]) -> gtk::Image {
    picture(svg_texture(svg, color, size).as_ref(), size, classes)
}

type TextureKey = (usize, usize, String, i32);

#[derive(Default)]
pub struct Textures {
    cache: RefCell<HashMap<TextureKey, Option<gdk::Texture>>>,
}

impl Textures {
    fn texture(&self, svg: &'static str, color: &str, size: i32) -> Option<gdk::Texture> {
        let key = (svg.as_ptr() as usize, svg.len(), color.to_owned(), size);
        if let Some(found) = self.cache.borrow().get(&key) {
            return found.clone();
        }
        let texture = svg_texture(svg, color, size);
        let mut cache = self.cache.borrow_mut();
        if cache.len() >= TEXTURE_CAPACITY {
            cache.clear();
        }
        cache.insert(key, texture.clone());
        texture
    }

    pub fn image(&self, svg: &'static str, color: &str, size: i32, classes: &[&str]) -> gtk::Image {
        picture(self.texture(svg, color, size).as_ref(), size, classes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOT: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="4"><rect width="4" height="4" fill="#bebebe"/></svg>"##;

    impl Textures {
        fn len(&self) -> usize {
            self.cache.borrow().len()
        }
    }

    #[test]
    fn textures_are_rendered_once_per_icon_color_and_size() {
        let textures = Textures::default();
        let first = textures.texture(DOT, "#ff0000", 4);
        let second = textures.texture(DOT, "#ff0000", 4);
        assert!(first.is_some());
        assert_eq!(first, second);
        textures.texture(DOT, "#00ff00", 4);
        textures.texture(DOT, "#00ff00", 8);
        assert_eq!(textures.len(), 3);
    }

    #[test]
    fn the_cache_stays_bounded() {
        let textures = Textures::default();
        for size in 1..100 {
            textures.texture(DOT, "#ff0000", size);
        }
        assert!(textures.len() <= TEXTURE_CAPACITY);
    }
}
