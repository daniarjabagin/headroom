import Adw from 'gi://Adw';
import Gtk from 'gi://Gtk';
import { percentLeft, windowLabel } from '../format.js';
import { _ } from '../i18n.js';
import { accountName } from '../providers.js';
import { hiddenWindowsAfter, hiddenWindowsPatch, isWindowHidden } from '../settings.js';
import { signInTarget } from './registry.js';
import { providerImage } from './widgets.js';

function subtitleOf(account) {
    const parts = [account.providerName, account.plan, account.label ? account.email : null];
    if (account.owner === 'headroom') parts.push(_('added in Headroom'));
    return parts.filter(Boolean).join(' · ');
}

function removeSubtitle(account) {
    if (account.owner === 'headroom') return _('Deletes the sign-in Headroom created for this account');
    return _('Stops showing this account. Its CLI stays signed in.');
}

function shapeOf(account) {
    return JSON.stringify([account.owner, account.windows.map(window => [window.id, window.label])]);
}

function iconButton(iconName, tooltip, onClick) {
    const button = new Gtk.Button({ icon_name: iconName, tooltip_text: tooltip, valign: Gtk.Align.CENTER });
    button.connect('clicked', () => onClick());
    return button;
}

export class AccountRow {
    constructor({ account, dir, client, onMove, onRemove, onSignIn }) {
        this._client = client;
        this._actions = { onMove, onRemove, onSignIn };
        this._syncing = false;
        this._children = [];
        this.id = account.id;
        this.widget = new Adw.ExpanderRow({ use_markup: false });
        const handle = new Gtk.Image({ icon_name: 'list-drag-handle-symbolic', css_classes: ['dim-label'] });
        this.widget.add_prefix(handle);
        this.widget.add_prefix(providerImage(dir, account.provider, 24));
        this._signIn = new Gtk.Button({ label: _('Sign in again…'), valign: Gtk.Align.CENTER, visible: false });
        this._signIn.connect('clicked', () => this._actions.onSignIn(this._account));
        this.widget.add_suffix(this._signIn);
        this._visible = new Gtk.Switch({ valign: Gtk.Align.CENTER, tooltip_text: _('Show in the panel and popup') });
        this._visible.connect('notify::active', () => this._onVisibleToggled());
        this.widget.add_suffix(this._visible);
    }

    update(account, settings) {
        this._account = account;
        this._syncing = true;
        this.widget.title = accountName(account);
        this.widget.subtitle = subtitleOf(account);
        this._visible.active = !account.hidden;
        this._signIn.visible = signInTarget(account, this._client.providers) !== null;
        if (this._shape !== shapeOf(account)) this._rebuild(account);
        this._syncChildren(account, settings);
        this._syncing = false;
    }

    _onVisibleToggled() {
        if (!this._syncing) this._client.setAccountHidden(this.id, !this._visible.active);
    }

    _rebuild(account) {
        for (const child of this._children) this.widget.remove(child);
        this._labelRow = this._labelEntry();
        this._windowRows = account.windows.map(window => this._windowRow(window));
        this._children = [this._labelRow, ...this._windowRows, this._positionRow(), this._removeRow(account)];
        for (const child of this._children) this.widget.add_row(child);
        this._shape = shapeOf(account);
    }

    _syncChildren(account, settings) {
        const editing = (this._labelRow.get_state_flags() & Gtk.StateFlags.FOCUS_WITHIN) !== 0;
        if (!editing && this._labelRow.text !== (account.label ?? '')) this._labelRow.text = account.label ?? '';
        account.windows.forEach((window, index) => {
            const row = this._windowRows[index];
            row.active = !isWindowHidden(settings.display, account.id, window.id);
            row.subtitle = window.remainingPercent === null ? _('No data yet') : percentLeft(window.remainingPercent);
        });
    }

    _labelEntry() {
        const row = new Adw.EntryRow({ title: _('Label'), show_apply_button: true });
        row.connect('apply', () => this._client.setAccountLabel(this.id, row.text.trim()));
        return row;
    }

    _windowRow(window) {
        const row = new Adw.SwitchRow({ title: windowLabel(window.id, window.label), use_markup: false });
        row.connect('notify::active', () => {
            const display = this._client.settings?.display;
            if (this._syncing || !display) return;
            const windows = hiddenWindowsAfter(display, this.id, window.id, !row.active);
            this._client.updateSettings(hiddenWindowsPatch(this.id, windows));
        });
        return row;
    }

    _positionRow() {
        const row = new Adw.ActionRow({ title: _('Position') });
        const buttons = new Gtk.Box({ css_classes: ['linked'], valign: Gtk.Align.CENTER });
        buttons.append(iconButton('go-up-symbolic', _('Move up'), () => this._actions.onMove(this.id, -1)));
        buttons.append(iconButton('go-down-symbolic', _('Move down'), () => this._actions.onMove(this.id, 1)));
        row.add_suffix(buttons);
        return row;
    }

    _removeRow(account) {
        const row = new Adw.ActionRow({ title: _('Remove from Headroom'), subtitle: removeSubtitle(account) });
        const button = new Gtk.Button({ label: _('Remove'), valign: Gtk.Align.CENTER });
        button.add_css_class('destructive-action');
        button.connect('clicked', () => this._actions.onRemove(this._account));
        row.add_suffix(button);
        return row;
    }
}
