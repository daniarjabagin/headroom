import Gdk from 'gi://Gdk';
import GObject from 'gi://GObject';
import Gtk from 'gi://Gtk';

const ABOVE = 'headroom-drop-above';
const BELOW = 'headroom-drop-below';
const DRAGGING = 'headroom-dragging';

export const DRAG_CSS = `
.${ABOVE} { box-shadow: inset 0 2px 0 0 @accent_color; }
.${BELOW} { box-shadow: inset 0 -2px 0 0 @accent_color; }
.${DRAGGING} { opacity: 0.45; }
`;

function rowIndex(list, widget) {
    let index = 0;
    for (let child = list.get_first_child(); child; child = child.get_next_sibling()) {
        if (child === widget) return index;
        if (child instanceof Gtk.ListBoxRow) index += 1;
    }
    return -1;
}

function clearMarks(widget) {
    widget.remove_css_class(ABOVE);
    widget.remove_css_class(BELOW);
}

export class RowDragger {
    constructor({ list, onDrop }) {
        this._list = list;
        this._onDrop = onDrop;
    }

    attach(widget, id) {
        widget.add_controller(this._source(widget, id));
        widget.add_controller(this._target(widget));
    }

    _source(widget, id) {
        const source = new Gtk.DragSource({ actions: Gdk.DragAction.MOVE });
        source.connect('prepare', () => Gdk.ContentProvider.new_for_value(id));
        source.connect('drag-begin', (_source, drag) => {
            const icon = Gtk.DragIcon.get_for_drag(drag);
            icon.child = new Gtk.Picture({ paintable: new Gtk.WidgetPaintable({ widget }), can_shrink: false });
            widget.add_css_class(DRAGGING);
        });
        source.connect('drag-end', () => widget.remove_css_class(DRAGGING));
        return source;
    }

    _target(widget) {
        const target = Gtk.DropTarget.new(GObject.TYPE_STRING, Gdk.DragAction.MOVE);
        target.connect('motion', (_target, _x, y) => {
            const below = y > widget.get_height() / 2;
            widget.add_css_class(below ? BELOW : ABOVE);
            widget.remove_css_class(below ? ABOVE : BELOW);
            return Gdk.DragAction.MOVE;
        });
        target.connect('leave', () => clearMarks(widget));
        target.connect('drop', (_target, id, _x, y) => {
            clearMarks(widget);
            this._drop(widget, id, y > widget.get_height() / 2);
            return true;
        });
        return target;
    }

    _drop(widget, id, below) {
        this._onDrop(id, rowIndex(this._list, widget) + (below ? 1 : 0));
    }
}
