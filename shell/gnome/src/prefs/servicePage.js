import Adw from 'gi://Adw';
import Gio from 'gi://Gio';
import Gtk from 'gi://Gtk';
import { _ } from '../i18n.js';
import { startService } from '../service.js';
import { pillButton, spinner } from './widgets.js';

export class ServicePage {
    constructor(dir) {
        this._cancellable = new Gio.Cancellable();
        this.page = new Adw.PreferencesPage({ title: _('Service'), icon_name: 'system-run-symbolic' });
        const file = dir.get_child('icons').get_child('headroom-symbolic.svg');
        this._status = new Adw.StatusPage({ paintable: Gtk.IconPaintable.new_for_file(file, 96, 1), vexpand: true });
        this._button = pillButton(_('Start Service'), true, () => this._start());
        this._spinner = spinner();
        this._error = new Gtk.Label({ wrap: true, css_classes: ['error'], visible: false });
        const box = new Gtk.Box({ orientation: Gtk.Orientation.VERTICAL, spacing: 12, halign: Gtk.Align.CENTER });
        for (const child of [this._button, this._spinner, this._error]) box.append(child);
        this._status.child = box;
        const group = new Adw.PreferencesGroup();
        group.add(this._status);
        this.page.add(group);
        this.showConnecting();
    }

    showConnecting() {
        this._status.title = _('Connecting to Headroom…');
        this._status.description = '';
        this._show({ button: false, spinner: true });
    }

    showStopped() {
        this._status.title = _("Headroom service isn't running");
        this._status.description = _('Settings live in the Headroom service. Start it to change them.');
        this._show({ button: true, spinner: false });
    }

    destroy() {
        this._cancellable.cancel();
    }

    _show({ button, spinner: busy }) {
        this._button.visible = button;
        this._spinner.visible = busy;
        this._error.visible = false;
    }

    async _start() {
        this._show({ button: false, spinner: true });
        try {
            await startService(this._cancellable);
        } catch (error) {
            if (this._cancellable.is_cancelled()) return;
            this._show({ button: true, spinner: false });
            this._error.label = error.message;
            this._error.visible = true;
        }
    }
}
