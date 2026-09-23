import Clutter from 'gi://Clutter';
import Pango from 'gi://Pango';
import * as Animation from 'resource:///org/gnome/shell/ui/animation.js';
import { agoText } from '../format.js';
import { providerInfo } from '../providers.js';
import { button, column, fileIcon, label, row, spacer, themeIcon } from '../widgets.js';
import { noticeRow } from './notice.js';
import { QuotaRow } from './quotaRow.js';
import { expandedRows, trendRow } from './usageRows.js';

function providerIcon(ctx, provider) {
    const info = providerInfo(provider);
    if (info.icon === null) return themeIcon('application-x-executable-symbolic', 'headroom-provider-icon tinted');
    return fileIcon(ctx.dir, info.icon, `headroom-provider-icon${info.tinted ? ' tinted' : ''}`);
}

export function accountTitle(account, showName) {
    const name = providerInfo(account.provider).name;
    if (!showName) return name;
    const who = account.label ?? account.email;
    return who ? `${name}: ${who}` : name;
}

function failedOffline(ctx, account) {
    return ctx.offline && account.error?.kind === 'network';
}

function outdatedTag(ctx, account) {
    const tag = label('Outdated', 'headroom-stale-tag');
    ctx.tooltips.attach(tag, () => account.updatedAt && `Last updated ${agoText(account.updatedAt, ctx.now())}`);
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
        ctx.tooltips.attach(icon, () => account.error?.message ?? 'Refresh failed');
        return icon;
    }
    return null;
}

function header(ctx, account, showName) {
    const actor = row({ style_class: 'headroom-section-header' });
    actor.add_child(providerIcon(ctx, account.provider));
    const title = label(accountTitle(account, showName), 'headroom-title', { y_align: Clutter.ActorAlign.END });
    title.clutter_text.ellipsize = Pango.EllipsizeMode.END;
    actor.add_child(title);
    if (account.plan) actor.add_child(label(account.plan, 'headroom-plan', { y_align: Clutter.ActorAlign.END }));
    const status = statusSlot(ctx, account);
    if (status) actor.add_child(status);
    actor.add_child(spacer());
    return actor;
}

function signedOutNotice(ctx, account) {
    const info = providerInfo(account.provider);
    const actions = [{ label: 'Retry', run: () => ctx.actions.refresh(account.id) }];
    if (info.signInCommand) actions.unshift({ label: 'Copy command', run: () => ctx.actions.copy(info.signInCommand) });
    return noticeRow({
        kind: 'signin',
        title: `Signed out of ${info.name}`,
        detail: info.signInCommand
            ? `Run "${info.signInCommand}" and sign in, then Retry`
            : 'Sign in again, then Retry',
        actions,
    });
}

function errorNotice(ctx, account) {
    return noticeRow({
        kind: 'error',
        title: `Couldn't refresh ${providerInfo(account.provider).name}`,
        detail: account.error?.message ?? null,
        actions: [{ label: 'Retry', run: () => ctx.actions.refresh(account.id) }],
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

function hasExtras(account) {
    return account.usage !== null || account.balances.length > 0;
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

    update(account) {
        this._account = account;
        account.windows.forEach((window, index) => this._rows[index].update(window));
    }

    tick(now) {
        for (const quotaRow of this._rows) quotaRow.tick(now);
    }

    _shape(account) {
        return {
            windows: account.windows.map(window => window.id),
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
        this.actor.add_child(header(this._ctx, account, showName));
        const card = column({ style_class: 'headroom-card', x_expand: true });
        for (const notice of noticeRows(this._ctx, account)) card.add_child(notice);
        const signedOut = account.status === 'signed_out';
        this._rows = signedOut ? [] : account.windows.map(window => new QuotaRow(this._ctx, window));
        for (const quotaRow of this._rows) card.add_child(quotaRow.actor);
        if (!signedOut) this._addUsage(card, account);
        this.actor.add_child(card);
    }

    _addUsage(card, account) {
        if (account.usage) card.add_child(trendRow(this._ctx, account.usage));
        if (!hasExtras(account)) return;
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
