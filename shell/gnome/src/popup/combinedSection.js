import Clutter from 'gi://Clutter';
import { breakdownEntries, combinedMeterState, combinedPercent, headerAccount } from '../combined.js';
import { capacityReading, isPooled, percentReading, windowLabel } from '../format.js';
import { column, label, row } from '../widgets.js';
import { AccountHeader } from './accountHeader.js';
import { QuotaRow } from './quotaRow.js';
import { SegmentedMeter } from './segmentedMeter.js';

function combinedLook(membersOf) {
    return {
        meter: () => new SegmentedMeter(),
        meterState: (window, display) => combinedMeterState(window, membersOf(), display),
        percent: (window, valueMode) => combinedPercent(window, valueMode),
        reading: (percent, window, valueMode) =>
            isPooled(window)
                ? capacityReading(percent, window.capacityPercent, valueMode)
                : percentReading(percent, valueMode),
    };
}

function breakdownLine(entry) {
    const line = row({ style_class: 'headroom-tip-table' });
    line.add_child(label(entry.name, 'headroom-tip-name', { x_expand: true }));
    line.add_child(label(`${entry.reading} · ${entry.reset}`, 'headroom-tip-figure'));
    return line;
}

function breakdownTooltip(ctx, window, members) {
    const actor = column({ style_class: 'headroom-tip' });
    const title = windowLabel(window.id, window.label);
    actor.add_child(label(title, 'headroom-tip-title', { x_align: Clutter.ActorAlign.START }));
    for (const entry of breakdownEntries(window, members, ctx.now(), ctx.display))
        actor.add_child(breakdownLine(entry));
    return actor;
}

function shape(card) {
    return {
        detail: headerAccount(card).plan,
        members: card.accountIds,
        windows: card.group.windows.map(window => [window.id, window.segments.map(segment => segment.accountId)]),
    };
}

export class CombinedSection {
    constructor(ctx, card) {
        this._ctx = ctx;
        this.actor = column({ style_class: 'headroom-section', x_expand: true });
        this._build(card);
    }

    get id() {
        return this._card.id;
    }

    get accountIds() {
        return this._card.accountIds;
    }

    get header() {
        return this._header.actor;
    }

    canUpdate(card) {
        return JSON.stringify(shape(card)) === this._shapeKey;
    }

    update(card) {
        this._card = card;
        this._header.update(headerAccount(card));
        card.group.windows.forEach((window, index) => this._rows[index]?.update(window));
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

    _build(card) {
        this._card = card;
        this._shapeKey = JSON.stringify(shape(card));
        this._header = new AccountHeader(this._ctx, headerAccount(card), false);
        this.actor.add_child(this._header.actor);
        const content = column({ style_class: 'headroom-card', x_expand: true, reactive: true });
        const look = combinedLook(() => this._card.members);
        this._rows = card.group.windows.map(window => new QuotaRow(this._ctx, window, look));
        for (const quotaRow of this._rows) {
            this._ctx.tooltips.attach(quotaRow.actor, () =>
                breakdownTooltip(this._ctx, quotaRow.window, this._card.members)
            );
            content.add_child(quotaRow.actor);
        }
        content.visible = this._rows.length > 0;
        this.actor.add_child(content);
    }
}
