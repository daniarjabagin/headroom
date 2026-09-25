import { dashboardCards } from '../combined.js';
import { enter, settle, STAGGER_MS } from '../motion.js';
import { mergeOrder, moveItem } from '../order.js';
import { showsName } from '../providers.js';
import { row, spacer } from '../widgets.js';
import { AccountSection } from './accountSection.js';
import { collapsedRow } from './collapsedRow.js';
import { CombinedSection } from './combinedSection.js';
import { SpendSection } from './spendCard.js';

const LEADING_ACTORS = 1;

function visibleAccounts(state) {
    return state.accounts.filter(account => !account.hidden);
}

function isCollapsed(card) {
    return card.kind === 'combined' ? card.group.collapsed : card.account.collapsed;
}

function cardSubject(card) {
    return card.kind === 'account' ? card.account : card;
}

function titleShowsName(ctx, card, accounts) {
    return card.kind === 'account' && !ctx.masked && showsName(card.account, accounts);
}

function sectionFor(ctx, card, accounts) {
    if (card.kind === 'combined') return new CombinedSection(ctx, card);
    return new AccountSection(ctx, card.account, titleShowsName(ctx, card, accounts));
}

function canUpdateSection(ctx, section, card, accounts) {
    if (section.id !== card.id) return false;
    if (card.kind === 'combined') return section instanceof CombinedSection && section.canUpdate(card);
    return section instanceof AccountSection && section.canUpdate(card.account, titleShowsName(ctx, card, accounts));
}

function topBar(trailing) {
    const actor = row({ style_class: 'headroom-top-bar', x_expand: true });
    actor.add_child(spacer());
    actor.add_child(trailing);
    return actor;
}

export function shownSpend(state) {
    return state.display.showSpend ? state.spend : null;
}

export class Dashboard {
    constructor(ctx, content, reorderer, refreshActor) {
        this._ctx = ctx;
        this._content = content;
        this._reorderer = reorderer;
        this._refreshActor = refreshActor;
        this.generation = 0;
        this._layoutKey = null;
        this._spendSection = null;
        this._sections = [];
        this._pinnedCount = 0;
        this._state = null;
    }

    get sections() {
        return this._sections;
    }

    isEmpty(state) {
        return visibleAccounts(state).length === 0 && !shownSpend(state);
    }

    renderState(state, layoutKey) {
        this._state = state;
        const accounts = visibleAccounts(state);
        const cards = dashboardCards(state, accounts);
        const pinned = cards.filter(card => !isCollapsed(card));
        const folded = cards.filter(isCollapsed);
        const expanded = this._ctx.moreExpanded;
        const shown = expanded ? [...pinned, ...folded] : pinned;
        const key = JSON.stringify([layoutKey, pinned.length, folded.length, expanded]);
        if (this._canUpdateInPlace(state, key, shown, accounts)) {
            this._spendSection?.update(state.spend);
            shown.forEach((card, index) => this._sections[index].update(cardSubject(card)));
            return;
        }
        this._rebuild(state, key, { pinned, folded, shown, accounts });
    }

    _canUpdateInPlace(state, key, cards, accounts) {
        if (this._layoutKey !== key) return false;
        if (Boolean(this._spendSection) !== Boolean(shownSpend(state))) return false;
        if (this._sections.length !== cards.length || (this._sections.length === 0 && !this._spendSection))
            return false;
        return cards.every((card, index) => canUpdateSection(this._ctx, this._sections[index], card, accounts));
    }

    _rebuild(state, key, { pinned, folded, shown, accounts }) {
        const spendState = shownSpend(state);
        const refresh = this.detachRefresh();
        const spend = spendState ? new SpendSection(this._ctx, spendState, refresh) : null;
        const sections = shown.map(card => sectionFor(this._ctx, card, accounts));
        const actors = [spend ? spend.actor : topBar(refresh), ...sections.slice(0, pinned.length).map(s => s.actor)];
        if (folded.length > 0)
            actors.push(collapsedRow(this._ctx, folded, this._ctx.moreExpanded, () => this._toggleMore()));
        actors.push(...sections.slice(pinned.length).map(section => section.actor));
        this.replace(actors);
        this._spendSection = spend;
        this._sections = sections;
        this._pinnedCount = pinned.length;
        this._layoutKey = key;
        this._reorderer.setSections(sections.slice(0, pinned.length));
    }

    _toggleMore() {
        this._ctx.moreExpanded = !this._ctx.moreExpanded;
        if (this._state) this._ctx.rerender();
    }

    replace(actors) {
        this.generation += 1;
        this._ctx.tooltips.hide();
        this._reorderer.setSections([]);
        if (this._content.contains(this._refreshActor)) this.detachRefresh();
        this._content.destroy_all_children();
        this._spendSection = null;
        this._sections = [];
        this._pinnedCount = 0;
        this._layoutKey = null;
        for (const actor of actors) this._content.add_child(actor);
    }

    detachRefresh() {
        this._refreshActor.get_parent()?.remove_child(this._refreshActor);
        return this._refreshActor;
    }

    tick(now) {
        for (const section of this._sections) section.tick(now);
    }

    needsSecondTicks(now) {
        return this._sections.some(section => section.needsSecondTicks(now));
    }

    playEntrance() {
        const motion = this._ctx.motion;
        this._content.get_children().forEach((actor, index) => enter(motion, actor, index));
        if (!motion.enabled) return;
        this._spendSection?.grow();
        this._sections.forEach((section, index) => section.grow((index + LEADING_ACTORS) * STAGGER_MS));
    }

    settleEntrance() {
        for (const actor of this._content.get_children()) settle(actor);
        for (const section of this._sections) section.settle();
    }

    drop(from, to, allIds) {
        const pinned = this._sections.slice(0, this._pinnedCount);
        const moved = pinned[from];
        const reordered = moveItem(pinned, from, to);
        this._sections = [...reordered, ...this._sections.slice(this._pinnedCount)];
        this._content.set_child_at_index(moved.actor, to + LEADING_ACTORS);
        return mergeOrder(
            allIds,
            this._sections.flatMap(section => section.accountIds)
        );
    }
}
