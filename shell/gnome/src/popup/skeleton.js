import St from 'gi://St';
import { pulse, stopPulse } from '../motion.js';
import { column, row, spacer } from '../widgets.js';

const SHIMMER_OPACITY = 115;
const SHIMMER_PERIOD_MS = 1800;
const LOADING_SECTIONS = [2, 3];

function block(size) {
    return new St.Widget({ style_class: `headroom-skeleton-block ${size}` });
}

function line(left, right) {
    const actor = row({ style_class: 'headroom-row-line', x_expand: true });
    actor.add_child(block(left));
    actor.add_child(spacer());
    actor.add_child(block(right));
    return actor;
}

function meterRow() {
    const actor = column({ style_class: 'headroom-quota-row headroom-skeleton-row', x_expand: true });
    actor.add_child(line('label', 'note'));
    actor.add_child(block('meter'));
    actor.add_child(line('reading', 'trailing'));
    return actor;
}

function shimmer(motion, actor) {
    actor.connect('notify::mapped', () => {
        if (actor.mapped) pulse(motion, actor, SHIMMER_OPACITY, SHIMMER_PERIOD_MS);
        else stopPulse(actor);
    });
    return actor;
}

export function skeletonRows(motion, count) {
    const rows = column({ style_class: 'headroom-skeleton', x_expand: true });
    for (let index = 0; index < count; index++) rows.add_child(meterRow());
    return shimmer(motion, rows);
}

function skeletonHeader(motion) {
    const header = row({ style_class: 'headroom-section-header headroom-skeleton' });
    header.add_child(block('icon'));
    header.add_child(block('title'));
    return shimmer(motion, header);
}

function skeletonSection(motion, rowCount) {
    const section = column({ style_class: 'headroom-section', x_expand: true });
    section.add_child(skeletonHeader(motion));
    const card = column({ style_class: 'headroom-card', x_expand: true });
    card.add_child(skeletonRows(motion, rowCount));
    section.add_child(card);
    return section;
}

export function loadingSections(motion) {
    return LOADING_SECTIONS.map(rowCount => skeletonSection(motion, rowCount));
}
