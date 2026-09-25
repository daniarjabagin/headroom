import Adw from 'gi://Adw';
import GLib from 'gi://GLib';
import Gtk from 'gi://Gtk';
import { _ } from '../i18n.js';
import { onboardingEntries } from './onboardingModel.js';
import { providerImage } from './widgets.js';

const isInstalled = program => GLib.find_program_in_path(program) !== null;

function suffixButton(label, onClick) {
    const button = new Gtk.Button({ label, valign: Gtk.Align.CENTER });
    button.connect('clicked', () => onClick());
    return button;
}

export class OnboardingRows {
    constructor({ dir, client, actions }) {
        this._dir = dir;
        this._client = client;
        this._actions = actions;
        this._rows = [];
        this._switches = new Map();
        this._shape = '';
        this._syncing = false;
        this.group = new Adw.PreferencesGroup({
            title: _('Here’s what we found'),
            description: _('Turn off anything you don’t want to track.'),
        });
        this._addRow = new Adw.ActionRow({ title: _('Add another account…'), activatable: true });
        this._addRow.add_prefix(new Gtk.Image({ icon_name: 'list-add-symbolic', pixel_size: 16, width_request: 32 }));
        this._addRow.add_suffix(new Gtk.Image({ icon_name: 'go-next-symbolic' }));
        this._addRow.connect('activated', () => actions.addAccount({}));
        this.group.add(this._addRow);
    }

    update(state) {
        const entries = onboardingEntries(state.accounts, this._client.providers ?? [], isInstalled);
        const shape = JSON.stringify(entries.map(entry => [entry.key, entry.kind, entry.title, entry.subtitle]));
        if (shape !== this._shape) this._rebuild(entries, shape);
        this._syncing = true;
        for (const entry of entries) if (entry.kind === 'account') this._switches.get(entry.key).active = entry.shown;
        this._syncing = false;
    }

    _rebuild(entries, shape) {
        for (const row of [...this._rows, this._addRow]) this.group.remove(row);
        this._switches.clear();
        this._rows = entries.map(entry => this._entryRow(entry));
        for (const row of [...this._rows, this._addRow]) this.group.add(row);
        this._shape = shape;
    }

    _entryRow(entry) {
        const row = new Adw.ActionRow({ title: entry.title, subtitle: entry.subtitle, use_markup: false });
        row.add_prefix(providerImage(this._dir, entry.provider, 32));
        if (entry.kind === 'account') row.add_suffix(this._accountSwitch(entry, row));
        if (entry.kind === 'signed_out')
            row.add_suffix(suffixButton(_('Sign In'), () => this._actions.signIn(entry.account)));
        if (entry.kind === 'found')
            row.add_suffix(suffixButton(_('Sign In'), () => this._actions.addAccount({ provider: entry.registry })));
        if (entry.kind === 'missing') {
            row.add_suffix(new Gtk.Switch({ valign: Gtk.Align.CENTER, active: false }));
            row.sensitive = false;
        }
        return row;
    }

    _accountSwitch(entry, row) {
        const toggle = new Gtk.Switch({ valign: Gtk.Align.CENTER, active: entry.shown });
        toggle.connect('notify::active', () => {
            if (!this._syncing) this._client.setAccountHidden(entry.accountId, !toggle.active);
        });
        row.activatable_widget = toggle;
        this._switches.set(entry.key, toggle);
        return toggle;
    }
}
