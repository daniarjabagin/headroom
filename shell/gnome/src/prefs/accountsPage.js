import Adw from 'gi://Adw';
import Gtk from 'gi://Gtk';
import { _, fill } from '../i18n.js';
import { moveItem } from '../order.js';
import { accountName } from '../providers.js';
import { AccountRow } from './accountRow.js';
import { AddAccountDialog } from './addAccountDialog.js';
import { ProgressProcess } from '../cli.js';
import { RowDragger } from './rowDragger.js';

function removalBody(account) {
    if (account.owner === 'headroom')
        return _('Headroom deletes the sign-in it created for this account. The account itself is not affected.');
    return fill(
        _(
            'Headroom will stop showing this account. The {provider} CLI stays signed in; you can sign in again through Headroom.'
        ),
        { provider: account.providerName }
    );
}

export class AccountsPage {
    constructor({ window, dir, client }) {
        this._window = window;
        this._dir = dir;
        this._client = client;
        this._rows = new Map();
        this._order = [];
        this.page = new Adw.PreferencesPage({ title: _('Accounts'), icon_name: 'system-users-symbolic' });
        this._list = new Gtk.ListBox({ selection_mode: Gtk.SelectionMode.NONE, css_classes: ['boxed-list'] });
        this._list.set_placeholder(this._placeholder());
        this._dragger = new RowDragger({ list: this._list, onDrop: (id, slot) => this._dropAt(id, slot) });
        const accounts = new Adw.PreferencesGroup({
            title: _('Accounts'),
            description: _('Drag to reorder. Hidden accounts keep updating but leave the panel and notifications.'),
        });
        accounts.add(this._list);
        this.page.add(accounts);
        this.page.add(this._addGroup());
    }

    update(state, settings) {
        const ids = state.accounts.map(account => account.id);
        if (JSON.stringify(ids) !== JSON.stringify(this._order)) this._rebuild(state.accounts);
        for (const account of state.accounts) this._rows.get(account.id).update(account, settings);
    }

    _placeholder() {
        const row = new Adw.ActionRow({
            title: _('No accounts yet'),
            subtitle: _('Sign in with a supported CLI, or add an account below.'),
        });
        row.add_css_class('dim-label');
        return row;
    }

    _rebuild(accounts) {
        const previous = this._rows;
        this._rows = new Map();
        this._list.remove_all();
        for (const account of accounts) {
            const row = previous.get(account.id) ?? this._createRow(account);
            this._rows.set(account.id, row);
            this._list.append(row.widget);
        }
        this._order = accounts.map(account => account.id);
    }

    _createRow(account) {
        const row = new AccountRow({
            account,
            dir: this._dir,
            client: this._client,
            onMove: (id, delta) => this._moveTo(id, this._order.indexOf(id) + delta),
            onRemove: target => this._confirmRemove(target),
        });
        this._dragger.attach(row.widget, row.id);
        return row;
    }

    _dropAt(id, slot) {
        const from = this._order.indexOf(id);
        this._moveTo(id, from < slot ? slot - 1 : slot);
    }

    _moveTo(id, index) {
        const from = this._order.indexOf(id);
        if (from < 0 || index < 0 || index >= this._order.length || index === from) return;
        const order = moveItem(this._order, from, index);
        this._rebuild(order.map(accountId => ({ id: accountId })));
        this._client.setAccountOrder(order);
    }

    _addGroup() {
        const group = new Adw.PreferencesGroup({
            title: _('Add Account'),
            description: _('Sign in through a CLI, paste an API key, or let Headroom find the account.'),
        });
        const row = new Adw.ActionRow({ title: _('Add account…'), activatable: true });
        row.add_prefix(new Gtk.Image({ icon_name: 'list-add-symbolic' }));
        row.add_suffix(new Gtk.Image({ icon_name: 'go-next-symbolic' }));
        row.connect('activated', () => this._openAddDialog());
        group.add(row);
        return group;
    }

    _openAddDialog() {
        new AddAccountDialog({
            dir: this._dir,
            providers: this._client.providers ?? [],
            providersError: this._client.providersError,
            onRestore: provider => this._client.restoreAccounts(provider),
        }).present(this._window);
    }

    _confirmRemove(account) {
        const name = accountName(account);
        const dialog = new Adw.AlertDialog({
            heading: fill(_('Remove {name}?'), { name }),
            body: removalBody(account),
        });
        dialog.add_response('cancel', _('Cancel'));
        dialog.add_response('remove', _('Remove'));
        dialog.set_response_appearance('remove', Adw.ResponseAppearance.DESTRUCTIVE);
        dialog.connect('response', (_dialog, response) => response === 'remove' && this._remove(account));
        dialog.present(this._window);
    }

    _remove(account) {
        let failure = null;
        const finish = error => this._toast(failure ?? error ?? _('Account removed'));
        try {
            new ProgressProcess(['accounts', 'remove', account.id, '--yes'], {
                onEvent: event => event.event === 'error' && (failure = event.message),
                onExit: finish,
            });
        } catch (error) {
            this._toast(error.message);
        }
    }

    _toast(title) {
        this._window.add_toast(new Adw.Toast({ title, timeout: 4 }));
    }
}
