use crate::popup_model::links::LinkKind;
use crate::ui::widgets::{Textures, icon};

const STATUS_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16"><path fill="#bebebe" d="M8 1.5a6.5 6.5 0 1 0 0 13 6.5 6.5 0 0 0 0-13Zm0 1.75a4.75 4.75 0 1 1 0 9.5 4.75 4.75 0 0 1 0-9.5ZM8 5.5a2.5 2.5 0 1 0 0 5 2.5 2.5 0 0 0 0-5Z"/></svg>"##;
const USAGE_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16"><path fill="#bebebe" d="M2 2h1.5v10.5H14V14H2Zm3 6h1.75v3.5H5Zm3-3h1.75v6.5H8Zm3 2h1.75v4.5H11Z"/></svg>"##;
const DASHBOARD_ICON: &str = "adw-external-link-symbolic";

pub fn link_icon(textures: &Textures, color: &str, kind: LinkKind, size: i32) -> gtk::Image {
    match kind {
        LinkKind::Status => textures.image(STATUS_SVG, color, size, &["headroom-link-glyph"]),
        LinkKind::Usage => textures.image(USAGE_SVG, color, size, &["headroom-link-glyph"]),
        LinkKind::Dashboard => icon(DASHBOARD_ICON, size, &["headroom-link-glyph"]),
    }
}
