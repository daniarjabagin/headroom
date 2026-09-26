import Clutter from 'gi://Clutter';
import GLib from 'gi://GLib';
import St from 'gi://St';
import { usesHour12 } from '../dates.js';
import { DesktopClock } from '../desktopClock.js';
import { _ } from '../i18n.js';
import { parseDisplay, supports06 } from '../settings.js';
import { isRefreshing } from '../state.js';
import { column } from '../widgets.js';
import { CardActions } from './cardActions.js';
import { Dashboard } from './dashboard.js';
import { FloatingMenu } from './floatingMenu.js';
import { Footer } from './footer.js';
import { LoginLauncher } from './loginLauncher.js';
import { MaskBanner } from './maskBanner.js';
import { Overlay } from './overlay.js';
import { RefreshButton } from './refreshButton.js';
import { RefreshControl } from './refreshControl.js';
import { Reorderer } from './reorder.js';
import { Sharer } from './share/sharer.js';
import { SheenClock } from './sheen.js';
import { loadingSections } from './skeleton.js';
import { SpendChoices } from './spendChoices.js';
import { emptyView, errorView, serviceView } from './statusViews.js';
import { Toast } from './toast.js';
import { Tooltips } from './tooltip.js';
import { UpdateRow } from './updateRow.js';

const SCROLLBAR_LINGER_MS = 800;
const ENTRANCE_WINDOW_MS = 400;
const COMPACT_CLASS = 'headroom-compact';

function applyOrder(state, order) {
    const rank = new Map(order.map((id, index) => [id, index]));
    state.accounts = [...state.accounts].sort(
        (a, b) => (rank.get(a.id) ?? order.length) - (rank.get(b.id) ?? order.length)
    );
}

function layoutKey(ctx, state) {
    const display = state.display;
    return [
        display.showSpend,
        display.showAccountSpend,
        display.showTrend,
        display.combineAccounts,
        display.density,
        ctx.masked,
        ctx.linksVersion,
        supports06(state),
    ];
}

export class PopupView {
    constructor({ dir, versionText, motion, actions }) {
        this._view = { kind: 'loading', state: null };
        this._entranceUntil = 0;
        this._pendingView = null;
        this._scrollbarTimeoutId = 0;
        this._requestedMask = false;
        this._links = new Map();
        this._linksRequested = false;
        this._tooltips = new Tooltips(motion);
        this._clock = new DesktopClock(() => this._rerender());
        this._refreshControl = new RefreshControl(() => actions.refreshNow());
        this._login = new LoginLauncher(message =>
            this._toast.show(_("Couldn't start sign-in"), message, 'dialog-warning')
        );
        this._ctx = this._createContext(dir, motion, actions);
        this._refreshButton = new RefreshButton(this._ctx);
        this._refreshControl.attach(this._refreshButton);
        this.actor = new St.Widget({
            style_class: 'headroom-popup',
            layout_manager: new Clutter.BinLayout(),
            x_expand: true,
            y_expand: true,
        });
        this._tooltips.setSideAnchor(this.actor);
        this._sheen.attach(this.actor);
        this._buildLayout(versionText);
    }

    _createContext(dir, motion, actions) {
        const ctx = {
            dir,
            motion,
            actions,
            display: parseDisplay(null),
            tooltips: this._tooltips,
            now: () => new Date(),
            expanded: new Map(),
            offline: false,
            masked: false,
            moreExpanded: false,
            linksVersion: 0,
            canReorder: () => this._reorderer.enabled,
            sheen: new SheenClock(motion),
            pressRefresh: () => this._refreshControl.press(),
            signIn: accountId => this._login.launch(accountId),
            hour12: () => usesHour12(ctx.display.timeFormat, this._clock.format),
            links: provider => this._links.get(provider) ?? null,
            providerStatus: () => this._view.state?.providerStatus ?? [],
            canWriteSettings: () => supports06(this._view.state) && Boolean(actions.updateDisplay),
            canChooseUnit: () => supports06(this._view.state),
            openCardMenu: (card, point) => this._cardActions.open(card, point),
            toast: (title, detail, icon) => this._toast.show(title, detail, icon),
            rerender: () => this._rerender(),
        };
        this._sheen = ctx.sheen;
        this._spendChoices = new SpendChoices(ctx, () => this._view.state);
        ctx.spendSettings = () => this._spendChoices.current();
        ctx.selectSpend = changes => this._spendChoices.select(changes);
        return ctx;
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
        this._banner = new MaskBanner(() => this._ctx.actions.showNumbersAnyway());
        this._footer = new Footer(this._ctx, versionText);
        this._updateRow = new UpdateRow(this._ctx);
        for (const actor of [this._banner.actor, this._scroll, this._updateRow.actor, this._footer.actor])
            main.add_child(actor);
        this.actor.add_child(main);
        this.actor.add_child(this._buildDragLayer());
        this._buildOverlay();
    }

    _buildDragLayer() {
        const dragLayer = new St.Widget({
            style_class: 'headroom-drag-layer',
            layout_manager: new Clutter.FixedLayout(),
            clip_to_allocation: true,
            x_expand: true,
            y_expand: true,
        });
        this._reorderer = new Reorderer({
            layer: dragLayer,
            motion: this._ctx.motion,
            onDrop: (from, to) => this._onDrop(from, to),
            onSettled: () => this._renderPending(),
        });
        this._dashboard = new Dashboard(this._ctx, this._content, this._reorderer, this._refreshButton.actor);
        return dragLayer;
    }

    _buildOverlay() {
        const overlay = new Overlay();
        this._palette = new St.Widget({ style_class: 'headroom-share-palette', width: 0, height: 0 });
        overlay.add(this._palette);
        this._ctx.menu = new FloatingMenu(this._ctx, overlay);
        this._toast = new Toast(this._ctx, overlay, () => this._bottomAnchor());
        this._cardActions = new CardActions(this._ctx, new Sharer(this._ctx, this._palette));
        this.actor.add_child(overlay.actor);
    }

    _bottomAnchor() {
        return this._updateRow.actor.visible ? this._updateRow.actor : this._footer.actor;
    }

    setTheme(themeClass) {
        this._tooltips.setTheme(themeClass);
    }

    render(view, { masked = false } = {}) {
        this._requestedMask = masked;
        if (this._reorderer.dragging) {
            this._pendingView = view;
            return;
        }
        this._view = view;
        this._syncContext(view);
        this._footer.update(view);
        this._updateRow.update(view.kind === 'ready' ? view.state.update : null);
        if (view.kind === 'ready') this._renderState(view.state);
        else this._dashboard.replace(this._statusView(view));
        this._refreshControl.setDaemonBusy(view.kind === 'ready' && isRefreshing(view.state));
        if (Date.now() < this._entranceUntil) this._dashboard.enterFresh();
    }

    _syncContext(view) {
        this._ctx.masked = this._requestedMask;
        this._banner.show(this._requestedMask);
        if (view.kind !== 'ready') {
            if (view.kind === 'unavailable') this._linksRequested = false;
            return;
        }
        this._ctx.display = view.state.display;
        if (view.state.display.density === 'compact') this.actor.add_style_class_name(COMPACT_CLASS);
        else this.actor.remove_style_class_name(COMPACT_CLASS);
        this._requestLinks();
    }

    _renderState(state) {
        if (this._dashboard.isEmpty(state)) {
            this._dashboard.replace([emptyView(this._ctx)]);
            return;
        }
        const offlineChanged = this._ctx.offline !== state.offline;
        this._ctx.offline = state.offline;
        if (offlineChanged) this._dashboard.replace([]);
        this._dashboard.renderState(state, layoutKey(this._ctx, state));
    }

    _rerender() {
        this.render(this._view, { masked: this._requestedMask });
    }

    async _requestLinks() {
        const list = this._ctx.actions.listProviders;
        if (this._linksRequested || !list) return;
        this._linksRequested = true;
        try {
            const providers = await list();
            if (!providers) return;
            this._links = new Map(providers.map(provider => [provider.id, provider.links ?? null]));
            this._ctx.linksVersion += 1;
            this._rerender();
        } catch (error) {
            logError(error, 'Headroom could not list provider links');
        }
    }

    setUpdateRun(run) {
        this._updateRow.setRun(run);
    }

    relabel() {
        this._footer.relabel();
        this._updateRow.relabel();
        this._refreshButton.relabel();
        this._banner.relabel();
        this._dashboard.replace([]);
        this._rerender();
    }

    _renderPending() {
        const view = this._pendingView;
        this._pendingView = null;
        if (view) this.render(view, { masked: this._requestedMask });
    }

    tick() {
        const now = this._ctx.now();
        this._dashboard.tick(now);
        this._footer.tick(now);
    }

    needsSecondTicks() {
        const now = this._ctx.now();
        return this._dashboard.needsSecondTicks(now) || this._footer.needsSecondTicks(now);
    }

    setMaxHeight(pixels) {
        this.actor.style = `max-height: ${pixels}px;`;
    }

    onOpen() {
        this._scroll.vadjustment.value = 0;
        this._hideScrollbar();
        this.tick();
        this._entranceUntil = Date.now() + ENTRANCE_WINDOW_MS;
        this._dashboard.playEntrance();
        this._sheen.start();
    }

    onClose() {
        this._sheen.stop();
        this._entranceUntil = 0;
        this._dashboard.settleEntrance();
        this._reorderer.cancel();
        this._ctx.menu.close(false);
        this._toast.hide();
        this._tooltips.hide();
        this._hideScrollbar();
    }

    destroy() {
        this._sheen.stop();
        this._login.destroy();
        this._reorderer.destroy();
        this._hideScrollbar();
        this._toast.hide();
        this._clock.destroy();
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

    _onDrop(from, to) {
        const order = this._dashboard.drop(
            from,
            to,
            this._view.state.accounts.map(account => account.id)
        );
        for (const view of [this._view, this._pendingView]) if (view?.state) applyOrder(view.state, order);
        this._ctx.actions.setOrder(order);
    }
}
