import Clutter from 'gi://Clutter';
import GLib from 'gi://GLib';
import St from 'gi://St';
import { enter, settle, STAGGER_MS } from '../motion.js';
import { mergeOrder, moveItem } from '../order.js';
import { showsName } from '../providers.js';
import { parseDisplay } from '../settings.js';
import { isRefreshing } from '../state.js';
import { column, row, spacer } from '../widgets.js';
import { AccountSection } from './accountSection.js';
import { Footer } from './footer.js';
import { RefreshButton } from './refreshButton.js';
import { RefreshControl } from './refreshControl.js';
import { Reorderer } from './reorder.js';
import { SpendSection } from './spendCard.js';
import { loadingSections } from './skeleton.js';
import { emptyView, errorView, serviceView } from './statusViews.js';
import { Tooltips } from './tooltip.js';
import { UpdateRow } from './updateRow.js';

const SCROLLBAR_LINGER_MS = 800;
const ENTRANCE_WINDOW_MS = 400;
const LEADING_ACTORS = 1;

function visibleAccounts(state) {
    return state.accounts.filter(account => !account.hidden);
}

function applyOrder(state, order) {
    const rank = new Map(order.map((id, index) => [id, index]));
    state.accounts = [...state.accounts].sort(
        (a, b) => (rank.get(a.id) ?? order.length) - (rank.get(b.id) ?? order.length)
    );
}

function layoutKey(display) {
    return JSON.stringify([display.showSpend, display.showAccountSpend, display.showTrend]);
}

function shownSpend(state) {
    return state.display.showSpend ? state.spend : null;
}

function topBar(trailing) {
    const actor = row({ style_class: 'headroom-top-bar', x_expand: true });
    actor.add_child(spacer());
    actor.add_child(trailing);
    return actor;
}

export class PopupView {
    constructor({ dir, versionText, motion, actions }) {
        this._view = { kind: 'loading', state: null };
        this._entranceUntil = 0;
        this._generation = 0;
        this._sections = [];
        this._spendSection = null;
        this._refreshControl = new RefreshControl(() => actions.refreshNow());
        this._layoutKey = null;
        this._pendingView = null;
        this._scrollbarTimeoutId = 0;
        this._tooltips = new Tooltips();
        this._ctx = {
            dir,
            motion,
            display: parseDisplay(null),
            canReorder: () => this._reorderer.enabled,
            tooltips: this._tooltips,
            now: () => new Date(),
            expanded: new Map(),
            offline: false,
            period: 'today',
            selectPeriod: period => (this._ctx.period = period),
            actions,
            pressRefresh: () => this._refreshControl.press(),
        };
        this._refreshButton = new RefreshButton(this._ctx);
        this._refreshControl.attach(this._refreshButton);
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
        this._updateRow = new UpdateRow(this._ctx);
        const dragLayer = new St.Widget({
            style_class: 'headroom-drag-layer',
            layout_manager: new Clutter.FixedLayout(),
            clip_to_allocation: true,
            x_expand: true,
            y_expand: true,
        });
        this._reorderer = new Reorderer({
            layer: dragLayer,
            onDrop: (from, to) => this._onDrop(from, to),
            onSettled: () => this._renderPending(),
        });
        main.add_child(this._scroll);
        main.add_child(this._updateRow.actor);
        main.add_child(this._footer.actor);
        this.actor.add_child(main);
        this.actor.add_child(dragLayer);
    }

    setTheme(themeClass) {
        this._tooltips.setTheme(themeClass);
    }

    render(view) {
        if (this._reorderer.dragging) {
            this._pendingView = view;
            return;
        }
        this._view = view;
        if (view.kind === 'ready') this._ctx.display = view.state.display;
        this._footer.update(view);
        this._updateRow.update(view.kind === 'ready' ? view.state.update : null);
        const generation = this._generation;
        if (view.kind === 'ready') this._renderState(view.state);
        else this._replaceContent(this._statusView(view));
        this._refreshControl.setDaemonBusy(view.kind === 'ready' && isRefreshing(view.state));
        if (generation !== this._generation && Date.now() < this._entranceUntil) this._playEntrance();
    }

    setUpdateRun(run) {
        this._updateRow.setRun(run);
    }

    relabel() {
        this._footer.relabel();
        this._updateRow.relabel();
        this._refreshButton.relabel();
        this._replaceContent([]);
        this.render(this._view);
    }

    _renderPending() {
        const view = this._pendingView;
        this._pendingView = null;
        if (view) this.render(view);
    }

    tick() {
        const now = this._ctx.now();
        for (const section of this._sections) section.tick(now);
        this._footer.tick(now);
    }

    needsSecondTicks() {
        const now = this._ctx.now();
        return this._sections.some(section => section.needsSecondTicks(now));
    }

    setMaxHeight(pixels) {
        this.actor.style = `max-height: ${pixels}px;`;
    }

    onOpen() {
        this._scroll.vadjustment.value = 0;
        this._hideScrollbar();
        this.tick();
        this._entranceUntil = Date.now() + ENTRANCE_WINDOW_MS;
        this._playEntrance();
    }

    onClose() {
        this._entranceUntil = 0;
        this._settleEntrance();
        this._reorderer.cancel();
        this._tooltips.hide();
        this._hideScrollbar();
    }

    destroy() {
        this._reorderer.destroy();
        this._hideScrollbar();
        this._scroll.vadjustment.disconnectObject(this);
        this._refreshControl.destroy();
        this._refreshButton.actor.destroy();
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
        if (view.kind === 'unavailable') return [serviceView(this._ctx, view)];
        if (view.kind === 'error') return [errorView(this._ctx, view.error)];
        return loadingSections(this._ctx.motion);
    }

    _playEntrance() {
        const motion = this._ctx.motion;
        this._content.get_children().forEach((actor, index) => enter(motion, actor, index));
        if (!motion.enabled) return;
        this._spendSection?.grow();
        this._sections.forEach((section, index) => section.grow((index + LEADING_ACTORS) * STAGGER_MS));
    }

    _settleEntrance() {
        for (const actor of this._content.get_children()) settle(actor);
        for (const section of this._sections) section.settle();
    }

    _renderState(state) {
        const accounts = visibleAccounts(state);
        if (accounts.length === 0 && !shownSpend(state)) {
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
        if (this._layoutKey !== layoutKey(state.display)) return false;
        if (Boolean(this._spendSection) !== Boolean(shownSpend(state))) return false;
        if (this._sections.length !== accounts.length || this._sections.length === 0) return false;
        return accounts.every(
            (account, index) =>
                this._sections[index].id === account.id &&
                this._sections[index].canUpdate(account, showsName(account, accounts))
        );
    }

    _rebuild(state, accounts) {
        const spendState = shownSpend(state);
        const refresh = this._detachedRefresh();
        const spend = spendState ? new SpendSection(this._ctx, spendState, this._ctx.period, refresh) : null;
        const sections = accounts.map(account => new AccountSection(this._ctx, account, showsName(account, accounts)));
        const leading = spend ? spend.actor : topBar(refresh);
        this._replaceContent([leading, ...sections.map(section => section.actor)]);
        this._spendSection = spend;
        this._sections = sections;
        this._layoutKey = layoutKey(state.display);
        this._reorderer.setSections(sections);
    }

    _replaceContent(actors) {
        this._generation += 1;
        this._tooltips.hide();
        this._reorderer.setSections([]);
        if (this._content.contains(this._refreshButton.actor)) this._detachedRefresh();
        this._content.destroy_all_children();
        this._spendSection = null;
        this._sections = [];
        this._layoutKey = null;
        for (const actor of actors) this._content.add_child(actor);
    }

    _detachedRefresh() {
        const actor = this._refreshButton.actor;
        actor.get_parent()?.remove_child(actor);
        return actor;
    }

    _onDrop(from, to) {
        const moved = this._sections[from];
        this._sections = moveItem(this._sections, from, to);
        this._content.set_child_at_index(moved.actor, to + LEADING_ACTORS);
        const order = mergeOrder(
            this._view.state.accounts.map(account => account.id),
            this._sections.map(section => section.id)
        );
        for (const view of [this._view, this._pendingView]) if (view?.state) applyOrder(view.state, order);
        this._ctx.actions.setOrder(order);
    }
}
