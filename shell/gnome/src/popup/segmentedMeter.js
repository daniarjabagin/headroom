import Clutter from 'gi://Clutter';
import GObject from 'gi://GObject';
import St from 'gi://St';
import { segmentLayout } from '../combined.js';
import { EASE, STANDARD_MS } from '../motion.js';

function clamp(value, min, max) {
    return Math.min(Math.max(value, min), max);
}

function childBox(x, y, width, height) {
    const box = new Clutter.ActorBox();
    box.set_origin(x, y);
    box.set_size(width, height);
    return box;
}

function segmentActors() {
    return {
        track: new St.Widget({ style_class: 'headroom-meter-track' }),
        fill: new St.Widget({ style_class: 'headroom-meter-fill' }),
        tick: new St.Widget({ style_class: 'headroom-meter-tick', visible: false }),
    };
}

export const SegmentedMeter = GObject.registerClass(
    {
        Properties: {
            blend: GObject.ParamSpec.double(
                'blend',
                'Blend',
                'Progress from old to new fills',
                GObject.ParamFlags.READWRITE,
                0,
                1,
                1
            ),
        },
    },
    class HeadroomSegmentedMeter extends St.Widget {
        _init() {
            super._init({ style_class: 'headroom-meter headroom-segmented-meter', x_expand: true });
            this._blend = 1;
            this._from = [];
            this._targets = [];
            this._ticks = [];
            this._parts = [];
        }

        get blend() {
            return this._blend;
        }

        set blend(value) {
            this._blend = value;
            this.notify('blend');
            this.queue_relayout();
        }

        update({ segments }, animate) {
            const shown = this._shownFractions();
            this._ensureParts(segments.length);
            this._targets = segments.map(segment => clamp(segment.fraction ?? 0, 0, 1));
            this._ticks = segments.map(segment => segment.tick);
            segments.forEach((segment, index) => {
                const part = this._parts[index];
                part.fill.style_class = `headroom-meter-fill ${segment.tone}`;
                part.tick.visible = segment.tick !== null;
            });
            this.remove_transition('blend');
            if (animate && this.mapped)
                this._blendFrom(
                    this._targets.map((_target, index) => shown[index] ?? 0),
                    0
                );
            else this._settleAt(this._targets);
        }

        grow(delay) {
            this.remove_transition('blend');
            this._blendFrom(
                this._targets.map(() => 0),
                delay
            );
        }

        settle() {
            this.remove_transition('blend');
            this._settleAt(this._targets);
        }

        _settleAt(targets) {
            this._from = [...targets];
            this.blend = 1;
        }

        _blendFrom(from, delay) {
            this._from = from;
            this.blend = 0;
            this.ease_property('blend', 1, { duration: STANDARD_MS, delay, mode: EASE });
        }

        _shownFractions() {
            return this._targets.map((target, index) => {
                const from = this._from[index] ?? target;
                return from + (target - from) * this._blend;
            });
        }

        _ensureParts(count) {
            while (this._parts.length > count) {
                const part = this._parts.pop();
                for (const actor of Object.values(part)) actor.destroy();
            }
            while (this._parts.length < count) {
                const part = segmentActors();
                for (const actor of Object.values(part)) this.add_child(actor);
                this._parts.push(part);
            }
        }

        vfunc_allocate(box) {
            this.set_allocation(box);
            const content = this.get_theme_node().get_content_box(box);
            const gap = this.get_theme_node().get_length('spacing');
            const layout = segmentLayout(this._parts.length, content.get_width(), Math.round(gap));
            const shown = this._shownFractions();
            layout.forEach((slot, index) => this._allocatePart(content, slot, index, shown[index] ?? 0));
        }

        _allocatePart(content, slot, index, fraction) {
            const part = this._parts[index];
            const [, trackHeight] = part.track.get_preferred_height(slot.width);
            const trackY = content.y1 + Math.round((content.get_height() - trackHeight) / 2);
            const x = content.x1 + slot.x;
            part.track.allocate(childBox(x, trackY, slot.width, trackHeight));
            const fillWidth = fraction <= 0 ? 0 : Math.max(trackHeight, Math.round(slot.width * fraction));
            part.fill.allocate(childBox(x, trackY, Math.min(fillWidth, slot.width), trackHeight));
            const tick = this._ticks[index];
            if (tick === null || tick === undefined) return;
            const [, tickWidth] = part.tick.get_preferred_width(-1);
            const tickX = clamp(Math.round(slot.width * tick - tickWidth / 2), 0, slot.width - tickWidth);
            part.tick.allocate(childBox(x + tickX, content.y1, tickWidth, content.get_height()));
        }
    }
);
