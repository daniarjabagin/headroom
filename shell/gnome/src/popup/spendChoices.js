import { displayPatch, supports06 } from '../settings.js';

const DISPLAY_KEYS = { period: 'spendPeriod', unit: 'spendUnit', breakdown: 'spendBreakdown' };

function displayChanges(changes) {
    return Object.fromEntries(Object.entries(changes).map(([key, value]) => [DISPLAY_KEYS[key], value]));
}

export class SpendChoices {
    constructor(ctx, stateOf) {
        this._ctx = ctx;
        this._stateOf = stateOf;
        this._local = { period: 'today', unit: 'cost', breakdown: 'models' };
    }

    current() {
        if (!this._ctx.canWriteSettings()) {
            const unit = supports06(this._stateOf()) ? this._local.unit : 'cost';
            return { ...this._local, unit };
        }
        const display = this._ctx.display;
        return { period: display.spendPeriod, unit: display.spendUnit, breakdown: display.spendBreakdown };
    }

    select(changes) {
        Object.assign(this._local, changes);
        if (!this._ctx.canWriteSettings()) {
            this._ctx.rerender();
            return;
        }
        const display = displayChanges(changes);
        this._ctx.actions.updateDisplay(display, displayPatch(display));
    }
}
