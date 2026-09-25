import { dashboardCards } from '../combined.js';
import { enter, settle, STAGGER_MS } from '../motion.js';
import { mergeOrder, moveItem } from '../order.js';
import { showsName } from '../providers.js';
import { row, spacer } from '../widgets.js';
import { AccountSection } from './accountSection.js';
import { collapsedNames, collapsedRow } from './collapsedRow.js';
import { CombinedSection } from './combinedSection.js';
import { cardShapeKey, planSections } from './sectionShape.js';
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
        this._state = null;
        this._fresh = new Set();
        this._clear();
    }

    get sections() {
        return this._sections;
    }

    isEmpty(state) {
        return visibleAccounts(state).length === 0 && !shownSpend(state);
    }

    renderState(state, layoutKey) {
        this._state = state;
        this._fresh.clear();
        const accounts = visibleAccounts(state);
        const cards = dashboardCards(state, accounts);
        const pinned = cards.filter(card => !isCollapsed(card));
        const folded = cards.filter(isCollapsed);
        const shown = this._ctx.moreExpanded ? [...pinned, ...folded] : pinned;
        this._syncHead(state, JSON.stringify(layoutKey));
        const changed = this._syncSections(shown, accounts);
        this._syncMore(folded);
        this._arrange(pinned.length);
        if (changed || this._pinnedCount !== pinned.length) this._setPinned(pinned.length);
    }

    _syncHead(state, key) {
        const spend = shownSpend(state);
        if (this._head && key === this._layoutKey && Boolean(spend) === Boolean(this._spendSection)) {
            this._spendSection?.update(spend);
            return;
        }
        this.replace([]);
        const refresh = this.detachRefresh();
        this._spendSection = spend ? new SpendSection(this._ctx, spend, refresh) : null;
        this._head = this._spendSection?.actor ?? topBar(refresh);
        this._layoutKey = key;
        this._fresh.add(this._head);
        this._content.add_child(this._head);
    }

    _syncSections(cards, accounts) {
        const wanted = cards.map(card => ({
            card,
            id: card.id,
            shapeKey: cardShapeKey(this._ctx, card, titleShowsName(this._ctx, card, accounts)),
        }));
        const plan = planSections(this._sections, wanted);
        for (const section of plan.dropped) section.actor.destroy();
        this._sections = wanted.map((entry, index) => {
            const section = plan.kept[index];
            if (section) {
                section.update(cardSubject(entry.card));
                return section;
            }
            const created = sectionFor(this._ctx, entry.card, accounts);
            this._fresh.add(created.actor);
            return created;
        });
        return plan.changed;
    }

    _syncMore(folded) {
        const expanded = this._ctx.moreExpanded;
        const key =
            folded.length === 0
                ? null
                : JSON.stringify([expanded, folded.map(card => card.id), collapsedNames(folded)]);
        if (key === this._moreKey) return;
        this._more?.destroy();
        this._more = key === null ? null : collapsedRow(this._ctx, folded, expanded, () => this._toggleMore());
        this._moreKey = key;
        if (this._more) this._fresh.add(this._more);
    }

    _arrange(pinnedCount) {
        const actors = this._sections.map(section => section.actor);
        const more = this._more ? [this._more] : [];
        const order = [this._head, ...actors.slice(0, pinnedCount), ...more, ...actors.slice(pinnedCount)];
        order.forEach((actor, index) => {
            if (actor.get_parent() !== this._content) this._content.insert_child_at_index(actor, index);
            else if (this._content.get_child_at_index(index) !== actor) this._content.set_child_at_index(actor, index);
        });
    }

    _setPinned(count) {
        this._pinnedCount = count;
        this._reorderer.setSections(this._sections.slice(0, count));
    }

    _toggleMore() {
        this._ctx.moreExpanded = !this._ctx.moreExpanded;
        if (this._state) this._ctx.rerender();
    }

    replace(actors) {
        this._fresh = new Set(actors);
        this._ctx.tooltips.hide();
        this._reorderer.setSections([]);
        if (this._content.contains(this._refreshActor)) this.detachRefresh();
        this._content.destroy_all_children();
        this._clear();
        for (const actor of actors) this._content.add_child(actor);
    }

    _clear() {
        this._head = null;
        this._spendSection = null;
        this._sections = [];
        this._more = null;
        this._moreKey = null;
        this._pinnedCount = 0;
        this._layoutKey = null;
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
        this._fresh = new Set();
        this._enter(() => true);
    }

    enterFresh() {
        const fresh = this._fresh;
        this._fresh = new Set();
        if (fresh.size > 0) this._enter(piece => fresh.has(piece));
    }

    _enter(isShown) {
        const motion = this._ctx.motion;
        this._content.get_children().forEach((actor, index) => {
            if (isShown(actor)) enter(motion, actor, index);
        });
        if (!motion.enabled) return;
        if (this._spendSection && isShown(this._spendSection.actor)) this._spendSection.grow();
        this._sections.forEach((section, index) => {
            if (isShown(section.actor)) section.grow((index + LEADING_ACTORS) * STAGGER_MS);
        });
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
        this._setPinned(this._pinnedCount);
        return mergeOrder(
            allIds,
            this._sections.flatMap(section => section.accountIds)
        );
    }
}
