import { lacksSubscription, subscriptionNote } from '../accountStatus.js';
import { _, fill } from '../i18n.js';
import { column } from '../widgets.js';
import { AccountHeader, failedOffline } from './accountHeader.js';
import { Expander } from './expander.js';
import { noticeRow } from './notice.js';
import { QuotaRow } from './quotaRow.js';
import { skeletonRows } from './skeleton.js';
import { ExtraRows, showsSpend, TrendRow } from './usageRows.js';

const SKELETON_ROWS = 2;

function signedOutNotice(ctx, account) {
    const actions = [{ label: _('Retry'), run: () => ctx.actions.refresh(account.id) }];
    if (account.owner === 'headroom')
        actions.unshift({ label: _('Settings'), run: () => ctx.actions.openPreferences() });
    return noticeRow({
        kind: 'signin',
        title: fill(_('Signed out of {provider}'), { provider: account.providerName }),
        detail: account.error?.message ?? _('Sign in again, then Retry'),
        actions,
    });
}

function noSubscriptionNotice(ctx, account) {
    return noticeRow({
        kind: 'warning',
        title: _('No active subscription'),
        detail: _("Limits aren't available for this account. Renew the plan or sign in with another account."),
        note: subscriptionNote(account.error),
        actions: [{ label: _('Retry'), run: () => ctx.actions.refresh(account.id) }],
    });
}

function errorNotice(ctx, account) {
    return noticeRow({
        kind: 'error',
        title: fill(_("Couldn't refresh {provider}"), { provider: account.providerName }),
        detail: account.error?.message ?? null,
        actions: [{ label: _('Retry'), run: () => ctx.actions.refresh(account.id) }],
    });
}

function showsErrorNotice(ctx, account) {
    return account.status === 'error' && !failedOffline(ctx, account);
}

function noticeRows(ctx, account) {
    if (account.status === 'signed_out') return [signedOutNotice(ctx, account)];
    if (lacksSubscription(account)) return [noSubscriptionNotice(ctx, account)];
    const rows = account.notices.map(notice =>
        noticeRow({ kind: notice.tone === 'critical' ? 'error' : 'warning', title: notice.text })
    );
    if (showsErrorNotice(ctx, account)) rows.unshift(errorNotice(ctx, account));
    return rows;
}

function showsLimits(account) {
    return account.status !== 'signed_out' && !lacksSubscription(account);
}

function shownWindows(account) {
    return account.windows.filter(window => !window.hidden);
}

function awaitingFirstData(account) {
    return (
        account.status === 'refreshing' &&
        account.updatedAt === null &&
        shownWindows(account).every(window => window.remainingPercent === null)
    );
}

function hasExtras(ctx, account) {
    return showsSpend(ctx, account) || account.balances.length > 0;
}

export class AccountSection {
    constructor(ctx, account, showName) {
        this._ctx = ctx;
        this._rows = [];
        this._trend = null;
        this._extras = null;
        this.actor = column({ style_class: 'headroom-section', x_expand: true });
        this._build(account, showName);
    }

    get id() {
        return this._account.id;
    }

    get header() {
        return this._header.actor;
    }

    canUpdate(account, showName) {
        return this._showName === showName && JSON.stringify(this._shape(account)) === this._shapeKey;
    }

    update(account) {
        this._account = account;
        this._header.update(account);
        shownWindows(account).forEach((window, index) => this._rows[index]?.update(window));
        if (account.usage) this._trend?.update(account.usage);
        this._extras?.update(account);
    }

    grow(delay) {
        for (const quotaRow of this._rows) quotaRow.grow(delay);
    }

    settle() {
        for (const quotaRow of this._rows) quotaRow.settle();
    }

    needsSecondTicks(now) {
        return this._rows.some(quotaRow => quotaRow.needsSecondTicks(now));
    }

    tick(now) {
        for (const quotaRow of this._rows) quotaRow.tick(now);
    }

    _shape(account) {
        const ctx = this._ctx;
        return {
            windows: shownWindows(account).map(window => window.id),
            skeleton: awaitingFirstData(account),
            signedOut: account.status === 'signed_out',
            noSubscription: lacksSubscription(account) ? subscriptionNote(account.error) : false,
            errorNotice: showsErrorNotice(ctx, account) ? account.error : null,
            notices: account.notices,
            plan: account.plan,
            label: account.label,
            email: account.email,
            trend: account.usage !== null && ctx.display.showTrend,
            spend: showsSpend(ctx, account),
            balances: account.balances.map(balance => [balance.id, balance.label, balance.kind]),
        };
    }

    _build(account, showName) {
        this._account = account;
        this._showName = showName;
        this._shapeKey = JSON.stringify(this._shape(account));
        this._header = new AccountHeader(this._ctx, account, showName);
        this.actor.add_child(this._header.actor);
        const card = column({ style_class: 'headroom-card', x_expand: true, reactive: true, track_hover: true });
        for (const notice of noticeRows(this._ctx, account)) card.add_child(notice);
        if (showsLimits(account)) this._addBody(card, account);
        card.visible = card.get_n_children() > 0;
        this.actor.add_child(card);
    }

    _addBody(card, account) {
        if (awaitingFirstData(account)) {
            card.add_child(skeletonRows(this._ctx.motion, Math.max(SKELETON_ROWS, account.windows.length)));
            return;
        }
        this._rows = shownWindows(account).map(window => new QuotaRow(this._ctx, window));
        for (const quotaRow of this._rows) card.add_child(quotaRow.actor);
        this._addUsage(card, account);
    }

    _addUsage(card, account) {
        if (account.usage && this._ctx.display.showTrend) {
            this._trend = new TrendRow(this._ctx, account.usage);
            card.add_child(this._trend.actor);
        }
        if (!hasExtras(this._ctx, account)) return;
        this._extras = new ExtraRows(this._ctx, account);
        if (this._rows.length === 0 && this._trend === null) {
            card.add_child(this._extras.actor);
            return;
        }
        const expander = new Expander(this._ctx, this._extras.actor, {
            expanded: this._ctx.expanded.get(account.id) === true,
            onToggled: expanded => this._ctx.expanded.set(this._account.id, expanded),
        });
        card.add_child(expander.toggle);
        card.add_child(expander.content);
    }
}
