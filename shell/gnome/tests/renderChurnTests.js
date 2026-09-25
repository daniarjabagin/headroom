import { dashboardCards } from '../src/combined.js';
import { cardShapeKey, planSections } from '../src/popup/sectionShape.js';
import { showsName } from '../src/providers.js';
import { sameValues } from '../src/sameValues.js';
import { parseState } from '../src/state.js';
import { check } from './check.js';

const RENDERS = 200;

function contextFor(state) {
    return {
        display: state.display,
        offline: state.offline,
        masked: false,
        providerStatus: () => state.providerStatus,
    };
}

function wantedSections(state) {
    const ctx = contextFor(state);
    const accounts = state.accounts.filter(account => !account.hidden);
    return dashboardCards(state, accounts).map(card => ({
        id: card.id,
        shapeKey: cardShapeKey(ctx, card, card.kind === 'account' && showsName(card.account, accounts)),
    }));
}

class ChurnCounter {
    constructor() {
        this.sections = [];
        this.created = 0;
        this.dropped = 0;
    }

    render(state) {
        const wanted = wantedSections(state);
        const plan = planSections(this.sections, wanted);
        this.dropped += plan.dropped.length;
        this.sections = wanted.map((entry, index) => {
            if (plan.kept[index]) return plan.kept[index];
            this.created += 1;
            return { ...entry };
        });
        return plan;
    }
}

function withRaw(json, change) {
    const raw = JSON.parse(json);
    change(raw);
    return parseState(JSON.stringify(raw));
}

function testIdenticalStates(json) {
    const counter = new ChurnCounter();
    counter.render(parseState(json));
    const initial = counter.created;
    let changedRenders = 0;
    for (let index = 0; index < RENDERS; index++) if (counter.render(parseState(json)).changed) changedRenders += 1;
    check(
        'identical states create no sections',
        [counter.created - initial, counter.dropped, changedRenders],
        [0, 0, 0]
    );
}

function testNumberOnlyChanges(json) {
    const counter = new ChurnCounter();
    counter.render(parseState(json));
    const initial = counter.created;
    for (let step = 1; step <= RENDERS; step++) {
        const state = withRaw(json, raw => {
            raw.accounts[0].windows[0].remaining_percent = step % 100;
            raw.accounts[0].windows[0].used_percent = 100 - (step % 100);
            raw.accounts[0].updated_at = new Date(Date.UTC(2026, 8, 25, 12, 0, step)).toISOString();
        });
        counter.render(state);
    }
    check('value updates happen in place', [counter.created - initial, counter.dropped], [0, 0]);
}

function testOneShapeChange(json) {
    const counter = new ChurnCounter();
    counter.render(parseState(json));
    const before = counter.sections.map(section => section.id);
    const state = withRaw(json, raw => {
        raw.accounts[0].notices = [{ tone: 'warning', text: 'Heads up' }];
    });
    const initial = counter.created;
    const plan = counter.render(state);
    check('one changed card rebuilds only itself', [counter.created - initial, plan.dropped.length], [1, 1]);
    check('dropped card is the changed one', plan.dropped[0]?.id, before[0]);
}

function testReorderKeepsSections(json) {
    const counter = new ChurnCounter();
    counter.render(parseState(json));
    const initial = counter.created;
    const plan = counter.render(withRaw(json, raw => raw.accounts.reverse()));
    check('reorder reuses every section', [counter.created - initial, plan.dropped.length, plan.changed], [0, 0, true]);
}

function testDensityRebuildsSectionsOnly(json) {
    const counter = new ChurnCounter();
    const first = counter.render(parseState(json));
    const compact = withRaw(json, raw => {
        raw.display = { ...(raw.display ?? {}), density: 'compact' };
    });
    const initial = counter.created;
    counter.render(compact);
    check('density rebuilds cards', counter.created - initial, first.kept.length);
}

function testPlanEdges() {
    const a = { id: 'a', shapeKey: '1' };
    const b = { id: 'b', shapeKey: '1' };
    check('empty plan', planSections([], []), { kept: [], dropped: [], changed: false });
    const removed = planSections([a, b], [{ id: 'a', shapeKey: '1' }]);
    check('removed card', [removed.kept[0] === a, removed.dropped, removed.changed], [true, [b], true]);
    const added = planSections(
        [a],
        [a, b].map(entry => ({ ...entry }))
    );
    check('added card', [added.kept[0] === a, added.kept[1], added.changed], [true, null, true]);
    check(
        'same values',
        [sameValues([1, 2], [1, 2]), sameValues([1], [1, 2]), sameValues([1, 2], [2, 1])],
        [true, false, false]
    );
}

export function testRenderChurn(json) {
    testIdenticalStates(json);
    testNumberOnlyChanges(json);
    testDensityRebuildsSectionsOnly(json);
}

export function testSectionPlan(json) {
    testOneShapeChange(json);
    testReorderKeepsSections(json);
    testPlanEdges();
}
