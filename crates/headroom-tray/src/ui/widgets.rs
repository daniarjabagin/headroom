use gtk::prelude::*;
use gtk::{gdk, glib};

use crate::assets::tinted_svg;

const SVG_RENDER_SCALE: i32 = 3;

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

pub fn svg_image(svg: &str, color: &str, size: i32, classes: &[&str]) -> gtk::Image {
    let image = gtk::Image::new();
    if let Some(texture) = svg_texture(svg, color, size) {
        image.set_paintable(Some(&texture));
    }
    image.set_pixel_size(size);
    for class in classes {
        image.add_css_class(class);
    }
    image
}
