import Clutter from 'gi://Clutter';
import { breakdownEntries, combinedMeterState, combinedPercent, headerAccount } from '../combined.js';
import { capacityReading, isPooled, percentReading, windowLabel } from '../format.js';
import { column, label, row } from '../widgets.js';
import { AccountHeader } from './accountHeader.js';
import { trailingLabel } from './limitTexts.js';
import { QuotaRow } from './quotaRow.js';
import { SegmentedMeter } from './segmentedMeter.js';
import { StatusNotice, statusShape } from './statusNotice.js';

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
    const now = ctx.now();
    breakdownEntries(window, members, now, ctx.display).forEach((entry, index) => {
        const reset = trailingLabel(window.segments[index], now, ctx.display.resetFormat, ctx.hour12());
        actor.add_child(breakdownLine({ ...entry, reset }));
    });
    return actor;
}

function shape(ctx, card) {
    return {
        status: statusShape(ctx, card.group.provider),
        detail: headerAccount(card).plan,
        members: card.accountIds,
        windows: card.group.windows.map(window => [window.id, window.segments.map(segment => segment.accountId)]),
    };
}

export class CombinedSection {
    constructor(ctx, card) {
        this._ctx = ctx;
        this._status = null;
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

    get card() {
        return this._card;
    }

    canUpdate(card) {
        return JSON.stringify(shape(this._ctx, card)) === this._shapeKey;
    }

    update(card) {
        this._card = card;
        this._header.update(headerAccount(card));
        this._status?.update();
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
        this._status?.update();
        for (const quotaRow of this._rows) quotaRow.tick(now);
    }

    _build(card) {
        this._card = card;
        this._shapeKey = JSON.stringify(shape(this._ctx, card));
        this._header = new AccountHeader(this._ctx, headerAccount(card), false);
        this._header.onSecondaryClick(point => this._ctx.openCardMenu(this._card, point));
        this.actor.add_child(this._header.actor);
        const content = column({ style_class: 'headroom-card', x_expand: true, reactive: true });
        if (statusShape(this._ctx, card.group.provider) !== null) {
            this._status = new StatusNotice(this._ctx, card.group.provider);
            content.add_child(this._status.actor);
        }
        const look = combinedLook(() => this._card.members);
        this._rows = card.group.windows.map(window => new QuotaRow(this._ctx, window, look));
        for (const quotaRow of this._rows) {
            this._ctx.tooltips.attach(quotaRow.actor, () =>
                this._ctx.masked ? null : breakdownTooltip(this._ctx, quotaRow.window, this._card.members)
            );
            content.add_child(quotaRow.actor);
        }
        content.visible = content.get_n_children() > 0;
        this.actor.add_child(content);
    }
}
