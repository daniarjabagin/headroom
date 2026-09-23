import Clutter from 'gi://Clutter';
import Pango from 'gi://Pango';
import * as Animation from 'resource:///org/gnome/shell/ui/animation.js';
import { agoText } from '../format.js';
import { _, fill } from '../i18n.js';
import { accountTitle, providerInfo } from '../providers.js';
import { button, column, label, providerIcon, row, spacer, themeIcon } from '../widgets.js';
import { noticeRow } from './notice.js';
import { QuotaRow } from './quotaRow.js';
import { expandedRows, showsSpend, trendRow } from './usageRows.js';

function failedOffline(ctx, account) {
    return ctx.offline && account.error?.kind === 'network';
}

function outdatedTag(ctx, account) {
    const tag = label(_('Outdated'), 'headroom-stale-tag');
    ctx.tooltips.attach(
        tag,
        () => account.updatedAt && fill(_('Last updated {ago}'), { ago: agoText(account.updatedAt, ctx.now()) })
    );
    return tag;
}

function statusSlot(ctx, account) {
    if (account.status === 'refreshing') {
        const spinner = new Animation.Spinner(12, { animate: true });
        spinner.add_style_class_name('headroom-header-spinner');
        spinner.y_align = Clutter.ActorAlign.CENTER;
        spinner.play();
        return spinner;
    }
    if (account.status === 'stale' || (account.status === 'error' && failedOffline(ctx, account)))
        return outdatedTag(ctx, account);
    if (account.status === 'error') {
        const icon = themeIcon('dialog-warning-symbolic', 'headroom-header-warning');
        ctx.tooltips.attach(icon, () => account.error?.message ?? _('Refresh failed'));
        return icon;
    }
    return null;
}

function dragGrip() {
    const grip = themeIcon('list-drag-handle-symbolic', 'headroom-drag-grip');
    grip.opacity = 0;
    return grip;
}

function header(ctx, account, showName) {
    const actor = row({ style_class: 'headroom-section-header', reactive: true, track_hover: true });
    actor.add_child(providerIcon(ctx.dir, account.provider, 'headroom-provider-icon'));
    const title = label(accountTitle(account, showName), 'headroom-title', { y_align: Clutter.ActorAlign.END });
    title.clutter_text.ellipsize = Pango.EllipsizeMode.END;
    actor.add_child(title);
    if (account.plan) actor.add_child(label(account.plan, 'headroom-plan', { y_align: Clutter.ActorAlign.END }));
    const status = statusSlot(ctx, account);
    if (status) actor.add_child(status);
    actor.add_child(spacer());
    const grip = dragGrip();
    actor.add_child(grip);
    actor.connect('notify::hover', () => (grip.opacity = actor.hover && ctx.canReorder() ? 255 : 0));
    return actor;
}

function signedOutNotice(ctx, account) {
    const info = providerInfo(account.provider);
    const actions = [{ label: _('Retry'), run: () => ctx.actions.refresh(account.id) }];
    if (info.signInCommand)
        actions.unshift({ label: _('Copy command'), run: () => ctx.actions.copy(info.signInCommand) });
    return noticeRow({
        kind: 'signin',
        title: fill(_('Signed out of {provider}'), { provider: info.name }),
        detail: info.signInCommand
            ? fill(_('Run "{command}" and sign in, then Retry'), { command: info.signInCommand })
            : _('Sign in again, then Retry'),
        actions,
    });
}

function errorNotice(ctx, account) {
    return noticeRow({
        kind: 'error',
        title: fill(_("Couldn't refresh {provider}"), { provider: providerInfo(account.provider).name }),
        detail: account.error?.message ?? null,
        actions: [{ label: _('Retry'), run: () => ctx.actions.refresh(account.id) }],
    });
}

function noticeRows(ctx, account) {
    if (account.status === 'signed_out') return [signedOutNotice(ctx, account)];
    const rows = account.notices.map(notice =>
        noticeRow({ kind: notice.tone === 'critical' ? 'error' : 'warning', title: notice.text })
    );
    if (account.status === 'error' && !failedOffline(ctx, account)) rows.unshift(errorNotice(ctx, account));
    return rows;
}

function hasExtras(ctx, account) {
    return showsSpend(ctx, account) || account.balances.length > 0;
}

function shownWindows(account) {
    return account.windows.filter(window => !window.hidden);
}

export class AccountSection {
    constructor(ctx, account, showName) {
        this._ctx = ctx;
        this.actor = column({ style_class: 'headroom-section', x_expand: true });
        this._build(account, showName);
    }

    get id() {
        return this._account.id;
    }

    canUpdate(account, showName) {
        return (
            this._showName === showName &&
            JSON.stringify(this._shape(account)) === JSON.stringify(this._shape(this._account))
        );
    }

    get header() {
        return this._header;
    }

    update(account) {
        this._account = account;
        shownWindows(account).forEach((window, index) => this._rows[index]?.update(window));
    }

    tick(now) {
        for (const quotaRow of this._rows) quotaRow.tick(now);
    }

    _shape(account) {
        return {
            windows: shownWindows(account).map(window => window.id),
            status: account.status,
            error: account.error,
            notices: account.notices,
            plan: account.plan,
            label: account.label,
            email: account.email,
            usage: account.usage,
            balances: account.balances,
        };
    }

    _build(account, showName) {
        this._account = account;
        this._showName = showName;
        this._header = header(this._ctx, account, showName);
        this.actor.add_child(this._header);
        const card = column({ style_class: 'headroom-card', x_expand: true });
        for (const notice of noticeRows(this._ctx, account)) card.add_child(notice);
        const signedOut = account.status === 'signed_out';
        this._rows = signedOut ? [] : shownWindows(account).map(window => new QuotaRow(this._ctx, window));
        for (const quotaRow of this._rows) card.add_child(quotaRow.actor);
        if (!signedOut) this._addUsage(card, account);
        card.visible = card.get_n_children() > 0;
        this.actor.add_child(card);
    }

    _addUsage(card, account) {
        if (account.usage && this._ctx.display.showTrend) card.add_child(trendRow(this._ctx, account.usage));
        if (!hasExtras(this._ctx, account)) return;
        const extra = expandedRows(this._ctx, account);
        const caret = themeIcon('pan-down-symbolic', 'headroom-caret-icon');
        const toggle = button(caret, 'headroom-caret', () => this._toggleExpanded(extra, caret));
        toggle.x_expand = true;
        this._applyExpanded(extra, caret);
        card.add_child(toggle);
        card.add_child(extra);
    }

    _toggleExpanded(extra, caret) {
        this._ctx.expanded.set(this._account.id, !this._ctx.expanded.get(this._account.id));
        this._applyExpanded(extra, caret);
    }

    _applyExpanded(extra, caret) {
        const expanded = this._ctx.expanded.get(this._account.id) === true;
        extra.visible = expanded;
        caret.icon_name = expanded ? 'pan-up-symbolic' : 'pan-down-symbolic';
    }
}
