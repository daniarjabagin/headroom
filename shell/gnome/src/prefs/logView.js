import Gtk from 'gi://Gtk';
import { _ } from '../i18n.js';

const LOG_HEIGHT = 140;

export class LogView {
    constructor() {
        this._view = new Gtk.TextView({
            editable: false,
            cursor_visible: false,
            monospace: true,
            wrap_mode: Gtk.WrapMode.WORD_CHAR,
            top_margin: 8,
            bottom_margin: 8,
            left_margin: 10,
            right_margin: 10,
        });
        const scroller = new Gtk.ScrolledWindow({
            child: this._view,
            min_content_height: LOG_HEIGHT,
            css_classes: ['card'],
        });
        this._end = this._view.buffer.create_mark(null, this._view.buffer.get_end_iter(), false);
        this.widget = new Gtk.Expander({ label: _('Details'), child: scroller, visible: false });
    }

    append(line) {
        const buffer = this._view.buffer;
        const prefix = buffer.get_char_count() > 0 ? '\n' : '';
        buffer.insert(buffer.get_end_iter(), `${prefix}${line}`, -1);
        this._view.scroll_to_mark(this._end, 0, false, 0, 1);
        this.widget.visible = true;
    }

    clear() {
        this._view.buffer.text = '';
        this.widget.visible = false;
    }
}
