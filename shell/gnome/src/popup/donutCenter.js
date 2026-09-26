import Pango from 'gi://Pango';
import St from 'gi://St';
import { centerTextRoom, fitCenterText, valueReach } from '../donutGeometry.js';

const PADDING_PX = 4;
const MIN_RATIO = 2 / 3;
const STEP = 0.5;
const DIGIT_FEATURES = 'tnum';

function measureAttributes() {
    const attributes = new Pango.AttrList();
    attributes.insert(Pango.attr_font_features_new(DIGIT_FEATURES));
    return attributes;
}

function captionHeight(caption) {
    return caption.visible ? caption.get_preferred_height(-1)[1] : 0;
}

export class CenterFit {
    constructor(donut, value, caption) {
        this._donut = donut;
        this._value = value;
        this._caption = caption;
        this._applied = null;
        this._attributes = measureAttributes();
        value.clutter_text.ellipsize = Pango.EllipsizeMode.END;
        value.connect('notify::text', () => this.fit());
        value.connect_after('style-changed', () => {
            this._applied = null;
            this.fit();
        });
    }

    fit() {
        const stage = this._value.get_stage();
        if (!stage) return;
        const size = this._donut.get_theme_node().get_width();
        if (size <= 0) return;
        const font = this._value.get_theme_node().get_font();
        const [ink, logical] = this._measure(font);
        const reach = valueReach({
            lineHeight: logical.height,
            captionHeight: captionHeight(this._caption),
            inkY: ink.y,
            inkHeight: ink.height,
        });
        const padding = PADDING_PX * St.ThemeContext.get_for_stage(stage).scale_factor;
        const room = centerTextRoom(size, reach, padding);
        const base = font.get_size() / Pango.SCALE;
        const fit = fitCenterText(logical.width, base, room, { minSize: base * MIN_RATIO, step: STEP });
        this._apply(font, fit.size);
        this._value.width = fit.clip ? Math.floor(room) : -1;
    }

    _measure(font) {
        const layout = this._value.clutter_text.create_pango_layout(this._value.text);
        layout.set_font_description(font);
        layout.set_attributes(this._attributes);
        return layout.get_pixel_extents();
    }

    _apply(font, size) {
        if (size === this._applied) return;
        this._applied = size;
        const scaled = font.copy();
        if (font.get_size_is_absolute()) scaled.set_absolute_size(size * Pango.SCALE);
        else scaled.set_size(Math.round(size * Pango.SCALE));
        this._value.clutter_text.set_font_description(scaled);
    }
}
