import Adw from 'gi://Adw';
import Gtk from 'gi://Gtk';
import { ProgressProcess } from '../cli.js';
import { _, fill } from '../i18n.js';
import { accountName } from '../providers.js';
import { supports06 } from '../settings.js';
import { AccountDetail } from './accountDetail.js';
import { AccountSidebar } from './accountSidebar.js';
import { AccountsLayout } from './accountsLayout.js';
import { orderAfterDrop, orderAfterMove, selectionAfter, sidebarEntries } from './accountsModel.js';
import { AddAccountDialog } from './addAccountDialog.js';
import { signInTarget } from './registry.js';

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
        this._accounts = [];
        this._selected = null;
        this._detail = null;
        this._subpage = null;
        this.page = new Adw.PreferencesPage({ title: _('Accounts'), icon_name: 'system-users-symbolic' });
        this._sidebar = new AccountSidebar({
            dir,
            onSelect: id => this._select(id),
            onActivate: id => this._activate(id),
            onDrop: (id, slot) => this._applyOrder(orderAfterDrop(this._sidebar.order, id, slot)),
        });
        this._layout = new AccountsLayout({ sidebar: this._sidebar.list, onAdd: () => this.openAddDialog() });
        this.page.add(this._layout.group);
        this._layout.widenPage();
    }

    update(state, settings) {
        this._state = state;
        this._settings = settings;
        const previous = this._sidebar.order;
        this._accounts = state.accounts;
        this._sidebar.update(sidebarEntries(state.accounts, settings.display, state.offline));
        this._layout.showEmpty(state.accounts.length === 0);
        this._select(selectionAfter(this._sidebar.order, this._selected, previous));
        this._syncDetails();
    }

    _select(id) {
        this._selected = id;
        this._sidebar.select(id);
        if (this._detail?.id === id) return;
        const account = this._accountOf(id);
        this._detail = account ? this._createDetail(account) : null;
        this._layout.showDetail(this._detail?.widget ?? null);
        this._syncDetails();
    }

    _activate(id) {
        this._select(id);
        const account = this._accountOf(id);
        if (!this._layout.narrow || !account) return;
        const detail = this._createDetail(account);
        const page = this._layout.detailPage(accountName(account), detail.widget);
        page.connect('hidden', () => this._subpage?.page === page && (this._subpage = null));
        this._subpage = { page, detail };
        this._syncDetails();
        this._window.push_subpage(page);
    }

    _syncDetails() {
        if (!this._state || !this._settings) return;
        for (const detail of [this._detail, this._subpage?.detail]) {
            const account = detail ? this._accountOf(detail.id) : null;
            if (account) detail.update(account, this._contextOf(account));
        }
        if (this._subpage && !this._accountOf(this._subpage.detail.id)) this._closeSubpage();
    }

    _closeSubpage() {
        this._subpage = null;
        this._window.pop_subpage();
    }

    _contextOf(account) {
        const provider = this._providerOf(account.provider);
        return {
            display: this._settings.display,
            offline: this._state.offline,
            canStar: supports06(this._state),
            signIn: signInTarget(account, this._client.providers),
            links: provider?.links ?? null,
            order: this._sidebar.order,
        };
    }

    _createDetail(account) {
        return new AccountDetail({
            dir: this._dir,
            client: this._client,
            account,
            actions: {
                onSignIn: target => this.signIn(target),
                onRemove: target => this._confirmRemove(target),
                onMove: (id, delta) => this._move(id, delta),
                onOpenLink: url => this._openLink(url),
            },
        });
    }

    _accountOf(id) {
        return this._accounts.find(account => account.id === id) ?? null;
    }

    _move(id, delta) {
        const order = this._sidebar.order;
        this._applyOrder(orderAfterMove(order, id, order.indexOf(id) + delta));
    }

    _applyOrder(order) {
        if (order === null) return;
        const byId = new Map(this._accounts.map(account => [account.id, account]));
        this.update({ ...this._state, accounts: order.map(id => byId.get(id)) }, this._settings);
        this._client.setAccountOrder(order);
    }

    _openLink(url) {
        new Gtk.UriLauncher({ uri: url }).launch(this._window, null, (launcher, result) => {
            try {
                launcher.launch_finish(result);
            } catch (error) {
                this._toast(error.message);
            }
        });
    }

    openAddDialog(options = {}) {
        new AddAccountDialog({
            dir: this._dir,
            providers: this._client.providers ?? [],
            providersError: this._client.providersError,
            onRestore: provider => this._client.restoreAccounts(provider),
            ...options,
        }).present(this._window);
    }

    signIn(account) {
        const target = signInTarget(account, this._client.providers);
        if (target) this.openAddDialog(target);
        else this.openAddDialog({ provider: this._providerOf(account.provider) });
    }

    _providerOf(id) {
        return (this._client.providers ?? []).find(provider => provider.id === id) ?? null;
    }

    _confirmRemove(account) {
        const dialog = new Adw.AlertDialog({
            heading: fill(_('Remove {name}?'), { name: accountName(account) }),
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
        this._window.add_toast(new Adw.Toast({ title, timeout: 4, use_markup: false }));
    }
}
