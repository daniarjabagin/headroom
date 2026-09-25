import Pango from 'gi://Pango';
import PangoCairo from 'gi://PangoCairo';

export function setColor(cr, color) {
    cr.setSourceRGBA(color.red / 255, color.green / 255, color.blue / 255, color.alpha / 255);
}

export function roundedRect(cr, x, y, width, height, radius) {
    const r = Math.min(radius, width / 2, height / 2);
    cr.newSubPath();
    cr.arc(x + width - r, y + r, r, -Math.PI / 2, 0);
    cr.arc(x + width - r, y + height - r, r, 0, Math.PI / 2);
    cr.arc(x + r, y + height - r, r, Math.PI / 2, Math.PI);
    cr.arc(x + r, y + r, r, Math.PI, (3 * Math.PI) / 2);
    cr.closePath();
}

export function fillRoundedRect(cr, box, radius, color) {
    if (box.width <= 0 || box.height <= 0) return;
    roundedRect(cr, box.x, box.y, box.width, box.height, radius);
    setColor(cr, color);
    cr.fill();
}

export function fillPolygon(cr, points, color) {
    cr.newPath();
    points.forEach(([x, y], index) => (index === 0 ? cr.moveTo(x, y) : cr.lineTo(x, y)));
    cr.closePath();
    setColor(cr, color);
    cr.fill();
}

function textAttributes(style) {
    const attributes = new Pango.AttrList();
    attributes.insert(Pango.attr_font_features_new('tnum'));
    if (style.spacing) attributes.insert(Pango.attr_letter_spacing_new(Math.round(style.spacing * Pango.SCALE)));
    return attributes;
}

export function textLayout(cr, text, style) {
    const layout = PangoCairo.create_layout(cr);
    const description = Pango.FontDescription.from_string(style.family);
    description.set_absolute_size(style.size * Pango.SCALE);
    description.set_weight(style.weight ?? Pango.Weight.NORMAL);
    layout.set_font_description(description);
    layout.set_attributes(textAttributes(style));
    layout.set_text(text, -1);
    return layout;
}

export function textSize(layout) {
    const [width, height] = layout.get_pixel_size();
    return { width, height, baseline: layout.get_baseline() / Pango.SCALE };
}

export function drawText(cr, text, style, { x, baseline, align = 'left' }) {
    const layout = textLayout(cr, text, style);
    const size = textSize(layout);
    const left = align === 'right' ? x - size.width : x;
    cr.moveTo(left, baseline - size.baseline);
    setColor(cr, style.color);
    PangoCairo.show_layout(cr, layout);
    return { ...size, x: left };
}
