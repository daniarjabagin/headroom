import {
    accountNotice,
    isRetrying,
    isSignedOut,
    lacksSubscription,
    noticeShape,
    recoveryActions,
    subscriptionNote,
} from '../accountStatus.js';
import { _, fill } from '../i18n.js';
import { column } from '../widgets.js';
import { AccountHeader } from './accountHeader.js';
import { Expander } from './expander.js';
import { noticeKind, noticeText } from '../notices.js';
import { CopyButton, Notice, noticeLine, noticeRow, RetryButton } from './notice.js';
import { QuotaRow } from './quotaRow.js';
import { skeletonRows } from './skeleton.js';
import { StatusNotice, statusShape } from './statusNotice.js';
import { ExtraRows, showsSpend, TrendRow } from './usageRows.js';

const SKELETON_ROWS = 2;
const NOTICE_KINDS = { signed_out: 'signin', no_subscription: 'warning', error: 'error' };

function runCommandText(account) {
    const command = account.recovery?.command;
    if (!command) return null;
    return fill(_('Run `{command}` in a terminal — Headroom picks it up automatically.'), { command });
}

function signedOutDetail(account) {
    const action = account.recovery?.action;
    if (action === 'cli_login') return runCommandText(account);
    if (action === 'retry')
        return fill(_('Sign in to {provider} again, then retry.'), { provider: account.providerName });
    return _('Sign in again through Headroom (Preferences → Accounts → Add account), or remove the account there.');
}

function signedOutTexts(account) {
    return {
        title: fill(_('Signed out of {provider}'), { provider: account.providerName }),
        detail: signedOutDetail(account),
        note: account.error?.message ?? null,
    };
}

function noSubscriptionTexts(account) {
    return {
        title: _('No active subscription'),
        detail: _("Limits aren't available for this account. Renew the plan or sign in with another account."),
        note: subscriptionNote(account.error),
    };
}

function errorTitle(account) {
    const provider = account.providerName;
    if (account.error?.kind === 'account_changed')
        return fill(_('{provider} is signed in to another account'), { provider });
    return fill(_("Couldn't refresh {provider}"), { provider });
}

function errorTexts(account) {
    return { title: errorTitle(account), detail: account.error?.message ?? null, note: runCommandText(account) };
}

const NOTICE_TEXTS = { signed_out: signedOutTexts, no_subscription: noSubscriptionTexts, error: errorTexts };

function noticeTexts(ctx, account) {
    const kind = accountNotice(account, ctx.offline);
    return kind === null ? null : NOTICE_TEXTS[kind](account);
}

function recoveryAction(ctx, account, retry, name) {
    if (name === 'sign_in')
        return { label: _('Sign in again…'), run: () => ctx.actions.openPreferences(), primary: true };
    if (name === 'copy_command') return { actor: new CopyButton(ctx.actions.copy, account.recovery.command).actor };
    return { actor: retry.actor };
}

function noticeActions(ctx, account, retry) {
    return recoveryActions(account).map(name => recoveryAction(ctx, account, retry, name));
}

function daemonNotice(notice) {
    const kind = noticeKind(notice.tone);
    const title = noticeText(notice.text);
    return kind === 'info' ? noticeLine(title) : noticeRow({ kind, title });
}

function isBlocked(account) {
    return isSignedOut(account) || lacksSubscription(account);
}

function shownWindows(account) {
    return account.windows.filter(window => !window.hidden);
}

function awaitingFirstData(account) {
    return (
        account.status === 'refreshing' &&
        account.error === null &&
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
        this._retry = null;
        this._notice = null;
        this._status = null;
        this.actor = column({ style_class: 'headroom-section', x_expand: true });
        this._build(account, showName);
    }

    get id() {
        return this._account.id;
    }

    get accountIds() {
        return [this._account.id];
    }

    get header() {
        return this._header.actor;
    }

    canUpdate(account, showName) {
        return this._showName === showName && JSON.stringify(this._shape(account)) === this._shapeKey;
    }

    get card() {
        return { kind: 'account', id: this._account.id, accountIds: [this._account.id], account: this._account };
    }

    update(account) {
        this._account = account;
        this._header.update(account);
        this._status?.update();
        this._retry?.setBusy(isRetrying(account));
        this._notice?.update(noticeTexts(this._ctx, account));
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
        this._status?.update();
        for (const quotaRow of this._rows) quotaRow.tick(now);
    }

    _shape(account) {
        const ctx = this._ctx;
        return {
            windows: shownWindows(account).map(window => window.id),
            skeleton: awaitingFirstData(account),
            notice: noticeShape(account, ctx.offline),
            status: statusShape(ctx, account.provider),
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
        this._header.onSecondaryClick(point => this._ctx.openCardMenu(this.card, point));
        this.actor.add_child(this._header.actor);
        const card = column({ style_class: 'headroom-card', x_expand: true, reactive: true });
        this._addStatus(card, account);
        if (isBlocked(account)) this._addNotice(card, account);
        else this._addLimits(card, account);
        card.visible = card.get_n_children() > 0;
        this.actor.add_child(card);
    }

    _addStatus(card, account) {
        if (statusShape(this._ctx, account.provider) === null) return;
        this._status = new StatusNotice(this._ctx, account.provider);
        card.add_child(this._status.actor);
    }

    _addNotice(card, account) {
        const kind = accountNotice(account, this._ctx.offline);
        if (kind === null) return;
        this._retry = new RetryButton(this._ctx.motion, () => this._ctx.actions.refresh(this._account.id));
        this._retry.setBusy(isRetrying(account));
        const actions = noticeActions(this._ctx, account, this._retry);
        this._notice = new Notice({ kind: NOTICE_KINDS[kind], actions, ...noticeTexts(this._ctx, account) });
        card.add_child(this._notice.actor);
    }

    _addLimits(card, account) {
        this._addNotice(card, account);
        for (const notice of account.notices) card.add_child(daemonNotice(notice));
        this._addBody(card, account);
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
