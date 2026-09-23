import Clutter from 'gi://Clutter';
import St from 'gi://St';
import { modelBreakdown } from '../breakdown.js';
import { exactTokens, exactUsd } from '../format.js';
import { _, fill } from '../i18n.js';
import { column, label, row } from '../widgets.js';

const PARTIAL_MARK = '*';

function textColumn(texts, styleClass, align) {
    const actor = column({ style_class: 'headroom-tip-column', x_expand: align === Clutter.ActorAlign.START });
    for (const text of texts) actor.add_child(label(text, styleClass, { x_align: align }));
    return actor;
}

function modelTable(rows) {
    const table = row({ style_class: 'headroom-tip-table' });
    const names = rows.map(entry => (entry.partial ? `${entry.name} ${PARTIAL_MARK}` : entry.name));
    table.add_child(textColumn(names, 'headroom-tip-name', Clutter.ActorAlign.START));
    table.add_child(
        textColumn(
            rows.map(entry => entry.tokens),
            'headroom-tip-figure',
            Clutter.ActorAlign.END
        )
    );
    table.add_child(
        textColumn(
            rows.map(entry => entry.cost),
            'headroom-tip-cost',
            Clutter.ActorAlign.END
        )
    );
    return table;
}

function totalLine(totals) {
    const line = fill(_('{cost} · {tokens} tokens'), {
        cost: exactUsd(totals.costMicros),
        tokens: exactTokens(totals.totalTokens),
    });
    return label(line, 'headroom-tip-total', { x_align: Clutter.ActorAlign.START });
}

export function modelTooltip(title, totals) {
    const breakdown = modelBreakdown(totals.models);
    if (breakdown === null) return null;
    const actor = column({ style_class: 'headroom-tip' });
    actor.add_child(label(title, 'headroom-tip-title', { x_align: Clutter.ActorAlign.START }));
    actor.add_child(modelTable(breakdown.rows));
    actor.add_child(new St.Widget({ style_class: 'headroom-tip-separator', x_expand: true }));
    actor.add_child(totalLine(totals));
    if (breakdown.partial)
        actor.add_child(
            label(`${PARTIAL_MARK} ${_('Partly unpriced, cost leaves it out')}`, 'headroom-tip-note', {
                x_align: Clutter.ActorAlign.START,
            })
        );
    return actor;
}
