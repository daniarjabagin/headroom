import Adw from 'gi://Adw';
import Gtk from 'gi://Gtk';
import { _ } from '../i18n.js';
import { accountName } from '../providers.js';
import { displayPatch, isStarred, starredAccountsPatch, starredAfter, supports06 } from '../settings.js';
import { group, switchRow } from './rows.js';
import { providerImage } from './widgets.js';

function cardSubtitle(account) {
    return [account.providerName, account.plan, account.label ? account.email : null].filter(Boolean).join(' · ');
}

function starRow(dir, account, onToggle) {
    const row = new Adw.ActionRow({ title: accountName(account), use_markup: false });
    row.add_prefix(providerImage(dir, account.provider, 32));
    const state = new Gtk.Label({ css_classes: ['dim-label'], valign: Gtk.Align.CENTER });
    const star = new Gtk.Button({ valign: Gtk.Align.CENTER, css_classes: ['flat', 'circular'] });
    star.connect('clicked', () => onToggle());
    row.add_suffix(state);
    row.add_suffix(star);
    row.activatable_widget = star;
    return { row, state, star };
}

export class CardsRows {
    constructor(client, dir) {
        this._client = client;
        this._dir = dir;
        this._entries = new Map();
        this._order = '';
        this._collapse = switchRow({
            title: _('Collapse unstarred accounts'),
            subtitle: _('They fold into one line in the popup and open on click'),
            onChange: value => client.updateSettings(displayPatch({ collapseUnstarred: value })),
        });
        this.group = group(
            _('Popup Cards'),
            [this._collapse.row],
            _('Star the accounts that always show their limits.')
        );
    }

    update(settings, state) {
        this.group.visible = supports06(state);
        this._collapse.set(settings.display.collapseUnstarred);
        const accounts = state.accounts.filter(account => !account.hidden);
        const order = JSON.stringify(accounts.map(account => [account.id, account.provider]));
        if (order !== this._order) this._rebuild(accounts, order);
        for (const account of accounts) this._sync(this._entries.get(account.id), account, settings.display);
    }

    _rebuild(accounts, order) {
        for (const entry of this._entries.values()) this.group.remove(entry.row);
        this._entries = new Map(
            accounts.map(account => [account.id, starRow(this._dir, account, () => this._toggle(account.id))])
        );
        for (const entry of this._entries.values()) this.group.add(entry.row);
        this._order = order;
    }

    _sync(entry, account, display) {
        const starred = isStarred(display, account.id);
        entry.row.title = accountName(account);
        entry.row.subtitle = cardSubtitle(account);
        entry.state.label = starred ? _('Always open') : _('On demand');
        entry.star.icon_name = starred ? 'starred-symbolic' : 'non-starred-symbolic';
        entry.star.tooltip_text = starred ? _('Show on demand') : _('Always show open');
    }

    _toggle(accountId) {
        const display = this._client.settings?.display;
        if (!display) return;
        const starred = !isStarred(display, accountId);
        this._client.updateSettings(starredAccountsPatch(starredAfter(display, accountId, starred)));
    }
}
