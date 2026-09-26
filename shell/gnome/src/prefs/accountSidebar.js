import Gtk from 'gi://Gtk';
import Pango from 'gi://Pango';
import { _ } from '../i18n.js';
import { RowDragger } from './rowDragger.js';
import { providerImage } from './widgets.js';

const MARK_CLASSES = {
    good: 'headroom-mark-good',
    warning: 'headroom-mark-warning',
    critical: 'headroom-mark-critical',
    neutral: 'headroom-mark-neutral',
};
const NOTICE_CLASSES = { signed_out: 'warning', no_subscription: 'warning', error: 'error' };

export const SIDEBAR_CSS = `
.headroom-mark { min-width: 8px; min-height: 8px; border-radius: 4px; }
.headroom-mark-good { background-color: @accent_bg_color; }
.headroom-mark-warning { background-color: @warning_bg_color; }
.headroom-mark-critical { background-color: @error_bg_color; }
.headroom-mark-neutral { background-color: alpha(currentColor, 0.25); }
.headroom-account-hidden { opacity: 0.6; }
`;

function textLabel(cssClasses) {
    return new Gtk.Label({ xalign: 0, ellipsize: Pango.EllipsizeMode.END, css_classes: cssClasses });
}

function dimIcon(iconName, tooltip) {
    return new Gtk.Image({ icon_name: iconName, css_classes: ['dim-label'], tooltip_text: tooltip, visible: false });
}

class SidebarRow {
    constructor(entry, dir) {
        this.id = entry.id;
        this._title = textLabel([]);
        this._subtitle = textLabel(['caption', 'dim-label']);
        this._star = dimIcon('starred-symbolic', _('Always show open'));
        this._hidden = dimIcon('view-conceal-symbolic', _('Hidden'));
        this._dot = new Gtk.Box({ css_classes: ['headroom-mark'], valign: Gtk.Align.CENTER });
        this._notice = new Gtk.Image({ icon_name: 'dialog-warning-symbolic', visible: false });
        this.widget = new Gtk.ListBoxRow({ child: this._content(entry, dir) });
    }

    _content(entry, dir) {
        const texts = new Gtk.Box({ orientation: Gtk.Orientation.VERTICAL, hexpand: true, valign: Gtk.Align.CENTER });
        texts.append(this._title);
        texts.append(this._subtitle);
        const box = new Gtk.Box({ spacing: 10, margin_top: 6, margin_bottom: 6, margin_start: 4, margin_end: 4 });
        for (const child of [providerImage(dir, entry.provider, 24), texts, this._star, this._hidden])
            box.append(child);
        box.append(this._dot);
        box.append(this._notice);
        return box;
    }

    update(entry) {
        this._title.label = entry.title;
        this._subtitle.label = entry.subtitle;
        this._subtitle.visible = entry.subtitle !== '';
        this._star.visible = entry.starred;
        this._hidden.visible = entry.hidden;
        this._syncMark(entry);
        this.widget.tooltip_text = entry.status ?? '';
        if (entry.hidden) this.widget.add_css_class('headroom-account-hidden');
        else this.widget.remove_css_class('headroom-account-hidden');
    }

    _syncMark(entry) {
        const notice = entry.mark.kind === 'notice';
        this._notice.visible = notice;
        this._dot.visible = !notice;
        this._notice.css_classes = notice ? [NOTICE_CLASSES[entry.mark.notice]] : [];
        this._dot.css_classes = notice ? ['headroom-mark'] : ['headroom-mark', MARK_CLASSES[entry.mark.tone]];
    }
}

export class AccountSidebar {
    constructor({ dir, onSelect, onActivate, onDrop }) {
        this._dir = dir;
        this._rows = new Map();
        this._order = [];
        this._syncing = false;
        this._handlers = { onSelect, onActivate };
        this.list = new Gtk.ListBox({ selection_mode: Gtk.SelectionMode.SINGLE, css_classes: ['navigation-sidebar'] });
        this.list.update_property([Gtk.AccessibleProperty.LABEL], [_('Accounts')]);
        this._dragger = new RowDragger({ list: this.list, onDrop });
        this.list.connect('row-selected', (_list, row) => this._selected(row));
        this.list.connect('row-activated', (_list, row) => this._handlers.onActivate(this._idOf(row)));
    }

    get order() {
        return this._order;
    }

    update(entries) {
        const ids = entries.map(entry => entry.id);
        if (JSON.stringify(ids) !== JSON.stringify(this._order)) this._rebuild(entries);
        for (const entry of entries) this._rows.get(entry.id).update(entry);
    }

    select(id) {
        const row = this._rows.get(id)?.widget ?? null;
        if (this.list.get_selected_row() === row) return;
        this._syncing = true;
        this.list.select_row(row);
        this._syncing = false;
    }

    _rebuild(entries) {
        const previous = this._rows;
        const selected = this._idOf(this.list.get_selected_row());
        this._syncing = true;
        this._rows = new Map();
        this.list.remove_all();
        for (const entry of entries) {
            const row = previous.get(entry.id) ?? this._createRow(entry);
            this._rows.set(entry.id, row);
            this.list.append(row.widget);
        }
        this._order = entries.map(entry => entry.id);
        this.list.select_row(this._rows.get(selected)?.widget ?? null);
        this._syncing = false;
    }

    _createRow(entry) {
        const row = new SidebarRow(entry, this._dir);
        this._dragger.attach(row.widget, row.id);
        return row;
    }

    _idOf(row) {
        if (!row) return null;
        return [...this._rows.values()].find(entry => entry.widget === row)?.id ?? null;
    }

    _selected(row) {
        const id = this._idOf(row);
        if (!this._syncing && id !== null) this._handlers.onSelect(id);
    }
}
