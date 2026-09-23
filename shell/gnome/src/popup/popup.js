import Clutter from 'gi://Clutter';
import GLib from 'gi://GLib';
import St from 'gi://St';
import { column } from '../widgets.js';
import { AccountSection } from './accountSection.js';
import { Footer } from './footer.js';
import { OptionsMenu } from './optionsMenu.js';
import { SpendSection } from './spendCard.js';
import { emptyView, errorView, loadingView, serviceView } from './statusViews.js';
import { Tooltips } from './tooltip.js';

const SCROLLBAR_LINGER_MS = 800;

function visibleAccounts(state) {
    return state.accounts.filter(account => !account.hidden);
}

function showsName(account, accounts) {
    return accounts.filter(other => other.provider === account.provider).length > 1;
}

export class PopupView {
    constructor({ dir, versionText, actions, settings }) {
        this._view = { kind: 'loading', state: null };
        this._sections = [];
        this._spendSection = null;
        this._scrollbarTimeoutId = 0;
        this._tooltips = new Tooltips();
        this._ctx = {
            dir,
            settings,
            tooltips: this._tooltips,
            now: () => new Date(),
            expanded: new Map(),
            offline: false,
            period: 'today',
            selectPeriod: period => (this._ctx.period = period),
            actions: { ...actions, toggleOptions: () => this._toggleOptions() },
        };
        this.actor = new St.Widget({
            style_class: 'headroom-popup',
            layout_manager: new Clutter.BinLayout(),
            x_expand: true,
            y_expand: true,
        });
        this._buildLayout(versionText);
    }

    _buildLayout(versionText) {
        const main = column({ x_expand: true, y_expand: true });
        this._content = column({ style_class: 'headroom-content', x_expand: true });
        this._scroll = new St.ScrollView({
            style_class: 'headroom-scroll',
            overlay_scrollbars: true,
            hscrollbar_policy: St.PolicyType.NEVER,
            vscrollbar_policy: St.PolicyType.AUTOMATIC,
            x_expand: true,
            y_expand: true,
        });
        this._scroll.child = this._content;
        this._scroll.vadjustment.connectObject('notify::value', () => this._revealScrollbar(), this);
        this._footer = new Footer(this._ctx, versionText);
        this._options = new OptionsMenu(this._ctx);
        main.add_child(this._scroll);
        main.add_child(this._footer.actor);
        this.actor.add_child(main);
        this.actor.add_child(this._options.actor);
    }

    render(view) {
        this._view = view;
        this._footer.update(view);
        if (view.kind === 'ready') this._renderState(view.state);
        else this._replaceContent([this._statusView(view)]);
    }

    tick() {
        const now = this._ctx.now();
        for (const section of this._sections) section.tick(now);
        this._footer.tick(now);
    }

    setMaxHeight(pixels) {
        this.actor.style = `max-height: ${pixels}px;`;
    }

    onOpen() {
        this._scroll.vadjustment.value = 0;
        this._hideScrollbar();
        this.tick();
    }

    onClose() {
        this._options.close();
        this._tooltips.hide();
        this._hideScrollbar();
    }

    destroy() {
        this._hideScrollbar();
        this._scroll.vadjustment.disconnectObject(this);
        this._tooltips.destroy();
        this.actor.destroy();
    }

    _revealScrollbar() {
        this._clearScrollbarTimeout();
        this._scroll.add_style_class_name('scrolling');
        this._scrollbarTimeoutId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, SCROLLBAR_LINGER_MS, () => {
            this._scrollbarTimeoutId = 0;
            this._scroll.remove_style_class_name('scrolling');
            return GLib.SOURCE_REMOVE;
        });
    }

    _hideScrollbar() {
        this._clearScrollbarTimeout();
        this._scroll.remove_style_class_name('scrolling');
    }

    _clearScrollbarTimeout() {
        if (this._scrollbarTimeoutId === 0) return;
        GLib.source_remove(this._scrollbarTimeoutId);
        this._scrollbarTimeoutId = 0;
    }

    _statusView(view) {
        if (view.kind === 'unavailable') return serviceView(this._ctx, view);
        if (view.kind === 'error') return errorView(this._ctx, view.error);
        return loadingView();
    }

    _renderState(state) {
        const accounts = visibleAccounts(state);
        if (accounts.length === 0 && !state.spend) {
            this._replaceContent([emptyView(this._ctx)]);
            return;
        }
        const offlineChanged = this._ctx.offline !== state.offline;
        this._ctx.offline = state.offline;
        if (!offlineChanged && this._canUpdateInPlace(state, accounts)) {
            this._spendSection?.update(state.spend);
            accounts.forEach((account, index) => this._sections[index].update(account));
            return;
        }
        this._rebuild(state, accounts);
    }

    _canUpdateInPlace(state, accounts) {
        if (Boolean(this._spendSection) !== Boolean(state.spend)) return false;
        if (this._sections.length !== accounts.length || this._sections.length === 0) return false;
        return accounts.every(
            (account, index) =>
                this._sections[index].id === account.id &&
                this._sections[index].canUpdate(account, showsName(account, accounts))
        );
    }

    _rebuild(state, accounts) {
        const spend = state.spend ? new SpendSection(this._ctx, state.spend, this._ctx.period) : null;
        const sections = accounts.map(account => new AccountSection(this._ctx, account, showsName(account, accounts)));
        this._replaceContent([...(spend ? [spend.actor] : []), ...sections.map(section => section.actor)]);
        this._spendSection = spend;
        this._sections = sections;
    }

    _replaceContent(actors) {
        this._tooltips.hide();
        this._content.destroy_all_children();
        this._spendSection = null;
        this._sections = [];
        for (const actor of actors) this._content.add_child(actor);
    }

    _toggleOptions() {
        const accounts = this._view.state?.accounts ?? [];
        this._options.toggle(this._view.kind === 'ready' ? accounts : []);
    }
}
