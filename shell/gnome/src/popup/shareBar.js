import Clutter from 'gi://Clutter';
import GObject from 'gi://GObject';
import St from 'gi://St';

const PERMILLE = 1000;
const MIN_PART_PX = 2;
const PART_GAP = 1;

function childBox(x, y, width, height) {
    const box = new Clutter.ActorBox();
    box.set_origin(x, y);
    box.set_size(width, height);
    return box;
}

function samePartsAs(current, next) {
    return (
        current.length === next.length &&
        current.every((part, index) => part.permille === next[index].permille && part.series === next[index].series)
    );
}

function partClass(series) {
    return series ? `headroom-share-part headroom-series-${series}` : 'headroom-share-part other';
}

export const ShareBar = GObject.registerClass(
    class HeadroomShareBar extends St.Widget {
        _init(styleClass = '') {
            super._init({ style_class: `headroom-share-bar ${styleClass}`.trim(), x_expand: true });
            this._parts = [];
        }

        setParts(parts) {
            const shown = parts.filter(part => part.permille > 0);
            if (samePartsAs(this._parts, shown)) return;
            this._parts = shown;
            this._syncChildren();
            this.queue_relayout();
        }

        _syncChildren() {
            const children = this.get_children();
            for (const child of children.slice(this._parts.length)) child.destroy();
            this._parts.forEach((part, index) => {
                const child = children[index] ?? new St.Widget();
                child.style_class = partClass(part.series);
                if (!children[index]) this.add_child(child);
            });
        }

        vfunc_get_preferred_width(_forHeight) {
            return [0, 0];
        }

        vfunc_allocate(box) {
            this.set_allocation(box);
            const content = this.get_theme_node().get_content_box(box);
            const width = content.get_width();
            const height = content.get_height();
            let x = content.x1;
            this.get_children().forEach((child, index) => {
                const partWidth = Math.max(MIN_PART_PX, Math.round((width * this._parts[index].permille) / PERMILLE));
                const shown = Math.min(partWidth, Math.max(0, content.x2 - x));
                child.allocate(childBox(x, content.y1, shown, height));
                x += shown + PART_GAP;
            });
        }
    }
);
