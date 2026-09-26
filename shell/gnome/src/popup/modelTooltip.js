import Clutter from 'gi://Clutter';
import Pango from 'gi://Pango';
import St from 'gi://St';
import { modelBreakdown, wholeShareText } from '../breakdown.js';
import { _ } from '../i18n.js';
import { exactUsd } from '../numbers.js';
import { column, label, row } from '../widgets.js';
import { MASK } from './mask.js';
import { ShareBar } from './shareBar.js';

const PARTIAL_MARK = '*';

function separator() {
    return new St.Widget({ style_class: 'headroom-pop-separator', x_expand: true });
}

function nameLine(entry, cost) {
    const line = row({ style_class: 'headroom-pop-line' });
    const name = label(entry.partial ? `${entry.name} ${PARTIAL_MARK}` : entry.name, 'headroom-pop-name', {
        x_expand: entry.detail === null,
    });
    name.clutter_text.ellipsize = Pango.EllipsizeMode.END;
    line.add_child(name);
    if (entry.detail) line.add_child(label(entry.detail, 'headroom-pop-dim', { x_expand: true }));
    line.add_child(label(cost, 'headroom-pop-strong'));
    return line;
}

function shareLine(entry, series, masked) {
    const line = row({ style_class: 'headroom-pop-line' });
    const bar = new ShareBar('headroom-pop-bar');
    bar.y_align = Clutter.ActorAlign.CENTER;
    bar.setParts(masked ? [] : [{ series, permille: entry.sharePermille }]);
    line.add_child(bar);
    const figures = masked ? MASK : `${wholeShareText(entry.sharePermille)} · ${entry.tokensText}`;
    line.add_child(label(figures, 'headroom-pop-dim headroom-pop-figures', { x_align: Clutter.ActorAlign.END }));
    return line;
}

function modelRow(entry, series, masked) {
    const actor = column({ style_class: 'headroom-pop-row' });
    actor.add_child(nameLine(entry, masked ? MASK : entry.cost));
    actor.add_child(shareLine(entry, series, masked));
    return actor;
}

function heading(title, total) {
    const line = row({ style_class: 'headroom-pop-line' });
    line.add_child(label(title, 'headroom-pop-title', { x_expand: true }));
    line.add_child(label(total, 'headroom-pop-strong'));
    return line;
}

function notes(totals, breakdown) {
    const lines = [];
    if (totals.modelsOther) lines.push(_('Models after the top 5 are folded into Other.'));
    lines.push(_('Estimated from local logs and public pricing.'));
    if (breakdown.partial) lines.push(`${PARTIAL_MARK} ${_('Partly unpriced, cost leaves it out')}`);
    const text = label(lines.join('\n'), 'headroom-pop-note', { x_align: Clutter.ActorAlign.START });
    text.clutter_text.line_wrap = true;
    return text;
}

export function modelTooltip(title, totals, { series = null, masked = false } = {}) {
    const breakdown = modelBreakdown(totals.models, totals.modelsOther, totals);
    if (breakdown === null) return null;
    const actor = column({ style_class: 'headroom-pop' });
    actor.add_child(heading(title, masked ? MASK : exactUsd(totals.costMicros)));
    actor.add_child(separator());
    for (const entry of breakdown.rows) actor.add_child(modelRow(entry, series, masked));
    actor.add_child(separator());
    actor.add_child(notes(totals, breakdown));
    return actor;
}
