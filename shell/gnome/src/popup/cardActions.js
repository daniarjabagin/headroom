import { isStarred, starredAccountsPatch, starredAfter } from '../settings.js';
import { cardMenuItems } from './cardMenu.js';

function cardIdentity(card) {
    return card.kind === 'combined'
        ? { provider: card.group.provider, providerName: card.group.providerName }
        : { provider: card.account.provider, providerName: card.account.providerName };
}

function starredList(display, accountIds, starred) {
    return accountIds.reduce(
        (list, id) => starredAfter({ ...display, starredAccounts: list }, id, starred),
        [...display.starredAccounts]
    );
}

export class CardActions {
    constructor(ctx, sharer) {
        this._ctx = ctx;
        this._sharer = sharer;
    }

    open(card, point) {
        const ctx = this._ctx;
        const { provider, providerName } = cardIdentity(card);
        const items = cardMenuItems({
            providerName,
            canHide: card.kind === 'account' && Boolean(ctx.actions.setHidden),
            canStar: ctx.canWriteSettings(),
            starred: card.accountIds.some(id => isStarred(ctx.display, id)),
            links: ctx.links(provider),
            canShare: !ctx.masked && this._sharer.canShare(card),
        });
        ctx.menu.open(items, point, item => this._run(card, item));
    }

    _run(card, item) {
        const actions = this._ctx.actions;
        if (item.url) actions.openUrl(item.url);
        else if (item.id === 'refresh') for (const id of card.accountIds) actions.refresh(id);
        else if (item.id === 'hide') actions.setHidden(card.account.id, true);
        else if (item.id === 'star') this._toggleStar(card);
        else if (item.id === 'share') this._sharer.shareImage(card);
        else if (item.id === 'copy') this._sharer.copyText(card);
    }

    _toggleStar(card) {
        const display = this._ctx.display;
        const starred = card.accountIds.some(id => isStarred(display, id));
        const list = starredList(display, card.accountIds, !starred);
        this._ctx.actions.updateDisplay({ starredAccounts: list }, starredAccountsPatch(list));
    }
}
