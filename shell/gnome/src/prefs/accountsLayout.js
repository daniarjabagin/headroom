import Adw from 'gi://Adw';
import Gtk from 'gi://Gtk';
import { _ } from '../i18n.js';
import { pillButton, wrapLabel } from './widgets.js';

const SIDEBAR_WIDTH = 220;
const NARROW_CONDITION = 'max-width: 500px';
const MIN_WIDTH = 280;
const MIN_HEIGHT = 320;
const PAGE_WIDTH = 860;

function addButton(onAdd) {
    const button = new Gtk.Button({
        child: new Adw.ButtonContent({ icon_name: 'list-add-symbolic', label: _('Add account…') }),
        tooltip_text: _('Sign in through a CLI, paste an API key, or let Headroom find the account.'),
        halign: Gtk.Align.START,
    });
    button.connect('clicked', () => onAdd());
    return button;
}

function emptyPage(onAdd) {
    return new Adw.StatusPage({
        icon_name: 'system-users-symbolic',
        title: _('No accounts yet'),
        description: _('Sign in with a supported CLI, or add an account below.'),
        child: pillButton(_('Add account…'), true, onAdd),
        css_classes: ['compact'],
        vexpand: true,
    });
}

function placeholder() {
    return new Gtk.Label({
        label: _('Select an account'),
        css_classes: ['dim-label'],
        vexpand: true,
        valign: Gtk.Align.CENTER,
    });
}

function sidebarColumn(sidebar, onAdd) {
    const frame = new Gtk.Frame({ child: sidebar, css_classes: ['view'] });
    const footer = wrapLabel(
        _('Drag to reorder. Hidden accounts keep updating but leave the panel and notifications.'),
        ['caption', 'dim-label']
    );
    footer.xalign = 0;
    footer.max_width_chars = 0;
    const column = new Gtk.Box({ orientation: Gtk.Orientation.VERTICAL, spacing: 8, width_request: SIDEBAR_WIDTH });
    for (const child of [frame, addButton(onAdd), footer]) column.append(child);
    return column;
}

export class AccountsLayout {
    constructor({ sidebar, onAdd }) {
        this.narrow = false;
        this._column = sidebarColumn(sidebar, onAdd);
        this._detail = new Gtk.Box({ orientation: Gtk.Orientation.VERTICAL, hexpand: true });
        this._detail.append(placeholder());
        const split = new Gtk.Box({ spacing: 18 });
        split.append(this._column);
        split.append(this._detail);
        this._stack = new Gtk.Stack({ vhomogeneous: false, hhomogeneous: false });
        this._stack.add_named(this._breakpointBin(split), 'split');
        this._stack.add_named(emptyPage(onAdd), 'empty');
        this.group = new Adw.PreferencesGroup();
        this.group.add(this._stack);
    }

    widenPage() {
        const clamp = this.group.get_ancestor(Adw.Clamp.$gtype);
        if (clamp) clamp.maximum_size = PAGE_WIDTH;
    }

    showEmpty(empty) {
        this._stack.visible_child_name = empty ? 'empty' : 'split';
    }

    showDetail(widget) {
        const current = this._detail.get_first_child();
        if (current) this._detail.remove(current);
        this._detail.append(widget ?? placeholder());
    }

    detailPage(title, widget) {
        widget.margin_top = 24;
        widget.margin_bottom = 24;
        widget.margin_start = 12;
        widget.margin_end = 12;
        const content = new Gtk.ScrolledWindow({
            hscrollbar_policy: Gtk.PolicyType.NEVER,
            vexpand: true,
            child: new Adw.Clamp({ child: widget }),
        });
        const view = new Adw.ToolbarView({ content });
        view.add_top_bar(new Adw.HeaderBar());
        return new Adw.NavigationPage({ title, child: view });
    }

    _breakpointBin(split) {
        const breakpoint = new Adw.Breakpoint({ condition: Adw.BreakpointCondition.parse(NARROW_CONDITION) });
        breakpoint.add_setter(this._detail, 'visible', false);
        breakpoint.add_setter(this._column, 'hexpand', true);
        breakpoint.connect('apply', () => (this.narrow = true));
        breakpoint.connect('unapply', () => (this.narrow = false));
        const bin = new Adw.BreakpointBin({ width_request: MIN_WIDTH, height_request: MIN_HEIGHT, child: split });
        bin.add_breakpoint(breakpoint);
        return bin;
    }
}
