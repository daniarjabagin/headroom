import Clutter from 'gi://Clutter';
import GObject from 'gi://GObject';
import St from 'gi://St';

const FILL_DURATION = 200;

function clamp(value, min, max) {
    return Math.min(Math.max(value, min), max);
}

function childBox(x, y, width, height) {
    const box = new Clutter.ActorBox();
    box.set_origin(x, y);
    box.set_size(width, height);
    return box;
}

export const Meter = GObject.registerClass(
    {
        Properties: {
            'shown-fraction': GObject.ParamSpec.double(
                'shown-fraction',
                'Shown fraction',
                'Currently drawn fill fraction',
                GObject.ParamFlags.READWRITE,
                0,
                1,
                0
            ),
        },
    },
    class HeadroomMeter extends St.Widget {
        _init() {
            super._init({ style_class: 'headroom-meter', x_expand: true });
            this._shownFraction = 0;
            this._tick = null;
            this._track = new St.Widget({ style_class: 'headroom-meter-track' });
            this._fill = new St.Widget({ style_class: 'headroom-meter-fill' });
            this._tickMark = new St.Widget({ style_class: 'headroom-meter-tick', visible: false });
            this.add_child(this._track);
            this.add_child(this._fill);
            this.add_child(this._tickMark);
        }

        get shown_fraction() {
            return this._shownFraction;
        }

        set shown_fraction(value) {
            this._shownFraction = value;
            this.notify('shown-fraction');
            this.queue_relayout();
        }

        update({ fraction, tone, tick }, animate) {
            const target = clamp(fraction ?? 0, 0, 1);
            this._fill.style_class = `headroom-meter-fill ${tone}`;
            this._tick = tick;
            this._tickMark.visible = tick !== null;
            this.remove_transition('shown-fraction');
            if (animate && this.mapped)
                this.ease_property('shown-fraction', target, {
                    duration: FILL_DURATION,
                    mode: Clutter.AnimationMode.EASE_OUT_CUBIC,
                });
            else this.shown_fraction = target;
        }

        vfunc_allocate(box) {
            this.set_allocation(box);
            const content = this.get_theme_node().get_content_box(box);
            const width = content.get_width();
            const [, trackHeight] = this._track.get_preferred_height(width);
            const [, tickWidth] = this._tickMark.get_preferred_width(-1);
            const trackY = content.y1 + Math.round((content.get_height() - trackHeight) / 2);
            this._track.allocate(childBox(content.x1, trackY, width, trackHeight));
            this._fill.allocate(childBox(content.x1, trackY, this._fillWidth(width, trackHeight), trackHeight));
            if (this._tick === null) return;
            const tickX = clamp(Math.round(width * this._tick - tickWidth / 2), 0, width - tickWidth);
            this._tickMark.allocate(childBox(content.x1 + tickX, content.y1, tickWidth, content.get_height()));
        }

        _fillWidth(width, trackHeight) {
            if (this._shownFraction <= 0) return 0;
            return Math.max(trackHeight, Math.round(width * this._shownFraction));
        }
    }
);
