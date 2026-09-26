import Adw from 'gi://Adw';
import Gtk from 'gi://Gtk';
import Pango from 'gi://Pango';
import { percentLeft } from '../format.js';
import { _ } from '../i18n.js';
import { accountName } from '../providers.js';
import {
    hiddenWindowsAfter,
    hiddenWindowsPatch,
    isStarred,
    isWindowHidden,
    starredAccountsPatch,
    starredAfter,
} from '../settings.js';
import { linksGroup, positionRow, removeRow, signInGroup, windowRow } from './accountDetailRows.js';
import { accountMark, canMove, detailSubtitle, noticeText } from './accountsModel.js';
import { providerImage } from './widgets.js';

const NOTICE_CLASSES = { signed_out: 'warning', no_subscription: 'warning', error: 'error' };

function label(cssClasses) {
    return new Gtk.Label({ xalign: 0, wrap: true, wrap_mode: Pango.WrapMode.WORD_CHAR, css_classes: cssClasses });
}

export class AccountDetail {
    constructor({ dir, client, account, actions }) {
        this._client = client;
        this._actions = actions;
        this._syncing = false;
        this._groups = [];
        this.id = account.id;
        this.widget = new Gtk.Box({ orientation: Gtk.Orientation.VERTICAL, spacing: 24 });
        this.widget.append(this._header(dir, account));
        this._identity = this._identityGroup();
        this.widget.append(this._identity);
    }

    update(account, context) {
        this._account = account;
        this._syncing = true;
        const shape = JSON.stringify(this._shapeOf(account, context));
        if (shape !== this._shape) this._rebuild(account, context, shape);
        this._syncHeader(account, context);
        this._syncIdentity(account, context);
        this._syncWindows(account, context.display);
        this._syncPosition(context);
        this._syncing = false;
    }

    _header(dir, account) {
        this._title = label(['title-3']);
        this._subtitle = label(['dim-label']);
        this._status = label([]);
        const texts = new Gtk.Box({ orientation: Gtk.Orientation.VERTICAL, spacing: 2, valign: Gtk.Align.CENTER });
        for (const child of [this._title, this._subtitle, this._status]) texts.append(child);
        const box = new Gtk.Box({ spacing: 12 });
        box.append(providerImage(dir, account.provider, 48));
        box.append(texts);
        return box;
    }

    _identityGroup() {
        this._visible = new Adw.SwitchRow({ title: _('Show in the panel and popup') });
        this._visible.connect('notify::active', () => this._guard(() => this._setHidden(!this._visible.active)));
        this._star = new Adw.SwitchRow({ title: _('Always show open') });
        this._star.connect('notify::active', () => this._guard(() => this._setStarred(this._star.active)));
        this._label = new Adw.EntryRow({ title: _('Label'), show_apply_button: true });
        this._label.connect('apply', () => this._client.setAccountLabel(this.id, this._label.text.trim()));
        const group = new Adw.PreferencesGroup();
        for (const row of [this._visible, this._star, this._label]) group.add(row);
        return group;
    }

    _shapeOf(account, context) {
        return [
            account.owner,
            account.windows.map(window => [window.id, window.label]),
            context.signIn !== null,
            context.links,
            context.order.length > 1,
        ];
    }

    _rebuild(account, context, shape) {
        for (const group of this._groups) this.widget.remove(group);
        this._windowRows = account.windows.map(window => windowRow(window, shown => this._setWindow(window.id, shown)));
        this._groups = [
            this._limitsGroup(),
            context.signIn ? signInGroup(account, () => this._actions.onSignIn(this._account)) : null,
            linksGroup(context.links, url => this._actions.onOpenLink(url)),
            this._manageGroup(account, context),
        ].filter(Boolean);
        for (const group of this._groups) this.widget.append(group);
        this._shape = shape;
    }

    _limitsGroup() {
        if (this._windowRows.length === 0) return null;
        const group = new Adw.PreferencesGroup({
            title: _('Limits'),
            description: _('Hidden limits leave the popup, the panel and notifications.'),
        });
        for (const row of this._windowRows) group.add(row);
        return group;
    }

    _manageGroup(account, context) {
        const group = new Adw.PreferencesGroup();
        this._position = context.order.length > 1 ? positionRow(delta => this._actions.onMove(this.id, delta)) : null;
        if (this._position) group.add(this._position.row);
        group.add(removeRow(account, () => this._actions.onRemove(this._account)));
        return group;
    }

    _syncHeader(account, context) {
        const mark = accountMark(account, context.display, context.offline);
        const status = mark.kind === 'notice' ? noticeText(account, mark.notice) : null;
        this._title.label = accountName(account);
        this._subtitle.label = detailSubtitle(account);
        this._status.label = status ?? '';
        this._status.visible = status !== null;
        this._status.css_classes = mark.kind === 'notice' ? [NOTICE_CLASSES[mark.notice]] : [];
    }

    _syncIdentity(account, context) {
        this._visible.active = !account.hidden;
        this._star.visible = context.canStar;
        this._star.active = isStarred(context.display, account.id);
        const editing = (this._label.get_state_flags() & Gtk.StateFlags.FOCUS_WITHIN) !== 0;
        if (!editing && this._label.text !== (account.label ?? '')) this._label.text = account.label ?? '';
    }

    _syncWindows(account, display) {
        account.windows.forEach((window, index) => {
            const row = this._windowRows[index];
            row.active = !isWindowHidden(display, account.id, window.id);
            row.subtitle = window.remainingPercent === null ? _('No data yet') : percentLeft(window.remainingPercent);
        });
    }

    _syncPosition(context) {
        if (!this._position) return;
        this._position.up.sensitive = canMove(context.order, this.id, -1);
        this._position.down.sensitive = canMove(context.order, this.id, 1);
    }

    _guard(run) {
        if (!this._syncing) run();
    }

    _setHidden(hidden) {
        this._client.setAccountHidden(this.id, hidden);
    }

    _setStarred(starred) {
        const display = this._client.settings?.display;
        if (display) this._client.updateSettings(starredAccountsPatch(starredAfter(display, this.id, starred)));
    }

    _setWindow(windowId, shown) {
        const display = this._client.settings?.display;
        if (this._syncing || !display) return;
        const windows = hiddenWindowsAfter(display, this.id, windowId, !shown);
        this._client.updateSettings(hiddenWindowsPatch(this.id, windows));
    }
}
