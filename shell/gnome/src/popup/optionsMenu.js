import Clutter from 'gi://Clutter';
import St from 'gi://St';
import { accountTitle } from './accountSection.js';
import { button, column, label, row, spacer, themeIcon } from '../widgets.js';

function menuItem(text, onActivate, leading = null) {
    const content = row({ style_class: 'headroom-menu-item-content', x_expand: true });
    if (leading) content.add_child(leading);
    content.add_child(label(text, 'headroom-menu-item-label', { x_expand: true }));
    const item = button(content, 'headroom-menu-item', onActivate);
    item.x_expand = true;
    return item;
}

function separator() {
    return new St.Widget({ style_class: 'headroom-menu-separator', x_expand: true });
}

function checkIcon(checked) {
    const icon = themeIcon('object-select-symbolic', 'headroom-menu-check');
    icon.opacity = checked ? 255 : 0;
    return icon;
}

export class OptionsMenu {
    constructor(ctx) {
        this._ctx = ctx;
        this.actor = column({
            style_class: 'headroom-options-scrim',
            reactive: true,
            x_expand: true,
            y_expand: true,
            visible: false,
        });
        this.actor.connect('button-press-event', (_actor, event) => this._onScrimPress(event));
        this._card = column({ style_class: 'headroom-options-menu', reactive: true });
        const anchor = row({ x_expand: true });
        anchor.add_child(spacer());
        anchor.add_child(this._card);
        this.actor.add_child(new St.Widget({ y_expand: true }));
        this.actor.add_child(anchor);
    }

    get isOpen() {
        return this.actor.visible;
    }

    toggle(accounts) {
        if (this.isOpen) this.close();
        else this.open(accounts);
    }

    open(accounts) {
        this._card.destroy_all_children();
        this._main = this._mainPage(accounts);
        this._accountsPage = this._buildAccountsPage(accounts);
        this._card.add_child(this._main);
        this._card.add_child(this._accountsPage);
        this._showPage(this._main);
        this.actor.show();
    }

    close() {
        this.actor.hide();
    }

    _onScrimPress(event) {
        if (!this._card.contains(global.stage.get_event_actor(event))) this.close();
        return Clutter.EVENT_STOP;
    }

    _showPage(page) {
        this._main.visible = page === this._main;
        this._accountsPage.visible = page === this._accountsPage;
    }

    _run(action) {
        this.close();
        action();
    }

    _page(items) {
        const page = column({ x_expand: true });
        for (const item of items) page.add_child(item);
        return page;
    }

    _mainPage(accounts) {
        const items = [
            menuItem('Refresh now', () => this._run(() => this._ctx.actions.refresh(''))),
            menuItem('Settings…', () => this._run(() => this._ctx.actions.openPreferences())),
        ];
        if (accounts.length > 0) {
            items.push(separator());
            items.push(menuItem('Hide accounts…', () => this._showPage(this._accountsPage)));
        }
        return this._page(items);
    }

    _buildAccountsPage(accounts) {
        const back = menuItem(
            'Accounts',
            () => this._showPage(this._main),
            themeIcon('go-previous-symbolic', 'headroom-menu-back')
        );
        return this._page([back, separator(), ...accounts.map(account => this._accountItem(account))]);
    }

    _accountItem(account) {
        const check = checkIcon(!account.hidden);
        const item = menuItem(
            accountTitle(account, true),
            () => {
                account.hidden = !account.hidden;
                check.opacity = account.hidden ? 0 : 255;
                this._ctx.actions.setHidden(account.id, account.hidden);
            },
            check
        );
        return item;
    }
}
