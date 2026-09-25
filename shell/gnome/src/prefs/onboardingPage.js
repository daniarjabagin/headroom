import Adw from 'gi://Adw';
import Gio from 'gi://Gio';
import Gtk from 'gi://Gtk';
import { usesHour12 } from '../dates.js';
import { _ } from '../i18n.js';
import { onboardingPatch } from '../settings.js';
import { OnboardingRows } from './onboardingRows.js';
import { PanelArt } from './panelArt.js';
import { ShortcutRow } from './privacyRows.js';
import { switchRow } from './rows.js';
import { serviceEnabled, setServiceEnabled } from './systemdUnit.js';
import { pillButton, wrapLabel } from './widgets.js';

const CONTENT_WIDTH = 440;

function column(children, marginTop) {
    const box = new Gtk.Box({
        orientation: Gtk.Orientation.VERTICAL,
        spacing: 8,
        width_request: CONTENT_WIDTH,
        halign: Gtk.Align.CENTER,
        margin_top: marginTop,
        margin_bottom: 16,
    });
    for (const child of children) box.append(child);
    return box;
}

function heading(text) {
    return new Gtk.Label({ label: text, css_classes: ['title-1'], wrap: true, justify: Gtk.Justification.CENTER });
}

function centered(text, cssClasses) {
    const label = wrapLabel(text, cssClasses);
    label.justify = Gtk.Justification.CENTER;
    return label;
}

function prefixIcon(iconName) {
    return new Gtk.Image({ icon_name: iconName, width_request: 32 });
}

function appTile(dir) {
    const file = dir.get_child('icons').get_child('headroom-symbolic.svg');
    const icon = new Gtk.Image({ gicon: new Gio.FileIcon({ file }), pixel_size: 64 });
    const tile = new Gtk.Box({ halign: Gtk.Align.CENTER, css_classes: ['headroom-app-tile'], margin_bottom: 10 });
    tile.append(icon);
    return tile;
}

function actionBar(children) {
    const box = new Gtk.Box({
        orientation: Gtk.Orientation.VERTICAL,
        spacing: 4,
        halign: Gtk.Align.CENTER,
        margin_top: 12,
        margin_bottom: 16,
    });
    for (const child of children) box.append(child);
    return box;
}

function spaced(widget, marginTop) {
    widget.margin_top = marginTop;
    return widget;
}

export class OnboardingPage {
    constructor({ dir, client, actions, clock, toast, onFinished }) {
        this._client = client;
        this._toast = toast;
        this._clock = clock;
        this._cancellable = new Gio.Cancellable();
        this._rows = new OnboardingRows({ dir, client, actions });
        this._art = new PanelArt();
        this._shortcut = new ShortcutRow(client, _('Keyboard shortcut'));
        this._shortcut.row.add_prefix(prefixIcon('input-keyboard-symbolic'));
        this._stack = new Gtk.Stack({ transition_type: Gtk.StackTransitionType.SLIDE_LEFT, vhomogeneous: false });
        this._stack.add_named(this._welcome(dir), 'welcome');
        this._stack.add_named(this._done(), 'done');
        this._buttons = this._buttonStack(onFinished);
        this._scroller = new Gtk.ScrolledWindow({ hscrollbar_policy: Gtk.PolicyType.NEVER, child: this._stack });
        const toolbar = new Adw.ToolbarView({ content: this._scroller });
        toolbar.add_top_bar(new Adw.HeaderBar({ show_title: false }));
        toolbar.add_bottom_bar(this._buttons);
        this.page = new Adw.NavigationPage({ title: _('Welcome to Headroom'), child: toolbar, can_pop: false });
        this.page.connect('shown', () => {
            this.page.get_root()?.set_focus(null);
            this._scroller.vadjustment.value = 0;
        });
        this._loadAutostart();
    }

    update(state, settings) {
        this._rows.update(state);
        this._shortcut.update(settings);
        this._art.update(state, settings, usesHour12(settings.display.timeFormat, this._clock.format));
    }

    destroy() {
        this._cancellable.cancel();
    }

    _buttonStack(onFinished) {
        const start = pillButton(_('Start'), true, () => this._start());
        const later = new Gtk.Button({ halign: Gtk.Align.CENTER, css_classes: ['flat'] });
        later.child = new Gtk.Label({ label: _('Choose later'), css_classes: ['accent'] });
        later.connect('clicked', () => onFinished());
        const stack = new Gtk.Stack({ transition_type: Gtk.StackTransitionType.CROSSFADE, vhomogeneous: false });
        stack.add_named(actionBar([start, later]), 'welcome');
        stack.add_named(actionBar([pillButton(_('Done'), true, () => onFinished())]), 'done');
        return stack;
    }

    _welcome(dir) {
        return column(
            [
                appTile(dir),
                heading(_('Welcome to Headroom')),
                centered(_('Your AI coding limits, right next to the clock.'), ['dim-label']),
                spaced(this._rows.group, 24),
            ],
            12
        );
    }

    _done() {
        this._autostart = switchRow({
            title: _('Start with the session'),
            subtitle: _('Headroom runs in the background after you log in'),
            onChange: value => this._setAutostart(value),
        });
        this._autostart.row.add_prefix(prefixIcon('view-refresh-symbolic'));
        const rows = new Adw.PreferencesGroup();
        rows.add(this._shortcut.row);
        rows.add(this._autostart.row);
        const body = _(
            'The ring next to the clock shows the limit that needs attention first. Click it for the details; settings are behind the gear at the bottom of the popup.'
        );
        return column(
            [
                this._art.widget,
                spaced(heading(_('Headroom lives in your top panel')), 24),
                centered(body, ['dim-label']),
                spaced(rows, 16),
                centered(_('Without the panel extension, look for the icon in the system tray.'), [
                    'dim-label',
                    'caption',
                ]),
            ],
            24
        );
    }

    _start() {
        this._client.updateSettings(onboardingPatch(true));
        this._stack.visible_child_name = 'done';
        this._buttons.visible_child_name = 'done';
        this._scroller.vadjustment.value = 0;
    }

    async _loadAutostart() {
        const enabled = await serviceEnabled(this._cancellable);
        if (this._cancellable.is_cancelled()) return;
        this._autostart.row.visible = enabled !== null;
        this._autostart.set(enabled === true);
    }

    async _setAutostart(enabled) {
        try {
            await setServiceEnabled(enabled, this._cancellable);
        } catch (error) {
            if (this._cancellable.is_cancelled()) return;
            this._autostart.set(!enabled);
            this._toast(error.message);
        }
    }
}
