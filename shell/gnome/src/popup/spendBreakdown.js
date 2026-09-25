import Clutter from 'gi://Clutter';
import Pango from 'gi://Pango';
import St from 'gi://St';
import {
    barParts,
    modelsTable,
    otherModelsText,
    otherProjectsText,
    preciseShareText,
    projectsTable,
} from '../breakdown.js';
import { _, fill, n_ } from '../i18n.js';
import { compactTokens, usd } from '../numbers.js';
import { projectLabel } from '../spendUnits.js';
import { column, label, row, spacer, textButton } from '../widgets.js';
import { popupIcon } from './icons.js';
import { MASK } from './mask.js';
import { ShareBar } from './shareBar.js';

const PROJECT_CHARS = 24;
const TABS = [
    ['models', () => _('Models')],
    ['projects', () => _('Projects')],
];

function caption(tab, count) {
    if (tab === 'projects') return fill(n_('{count} project', '{count} projects', count), { count });
    return fill(n_('{count} model', '{count} models', count), { count });
}

function rowName(ctx, tab, entry) {
    if (entry.other !== null) return _('Other');
    if (tab === 'projects') return ctx.masked ? MASK : projectLabel(entry.name, PROJECT_CHARS);
    return entry.name;
}

function otherText(tab, entry) {
    if (entry.other === null) return null;
    return tab === 'projects' ? otherProjectsText(entry.other) : otherModelsText(entry.other);
}

function marker(ctx, tab, entry) {
    if (tab === 'projects') return popupIcon(ctx.dir, 'folder', 'headroom-breakdown-folder');
    const series = entry.parts[0]?.series;
    return new St.Widget({
        style_class: `headroom-legend-dot headroom-series-${series ?? 'other-0'}`,
        y_align: Clutter.ActorAlign.CENTER,
    });
}

class BreakdownRow {
    constructor(ctx, tab, entry) {
        this._ctx = ctx;
        this.actor = column({ style_class: 'headroom-breakdown-row headroom-hover-chip', reactive: true });
        const line = row({ style_class: 'headroom-breakdown-line' });
        line.add_child(marker(ctx, tab, entry));
        const name = label(rowName(ctx, tab, entry), 'headroom-breakdown-name');
        name.clutter_text.ellipsize = tab === 'projects' ? Pango.EllipsizeMode.MIDDLE : Pango.EllipsizeMode.END;
        line.add_child(name);
        const detail = otherText(tab, entry);
        if (detail) line.add_child(label(detail, 'headroom-breakdown-share'));
        line.add_child(spacer());
        this._share = label('', 'headroom-breakdown-share');
        this._value = label('', 'headroom-breakdown-value', { x_align: Clutter.ActorAlign.END });
        line.add_child(this._share);
        line.add_child(this._value);
        this._bar = new ShareBar('headroom-breakdown-bar');
        this.actor.add_child(line);
        this.actor.add_child(this._bar);
    }

    update(entry, period, unit) {
        const masked = this._ctx.masked;
        this._share.text = masked ? MASK : preciseShareText(entry.sharePermille);
        this._value.text = masked ? MASK : unit === 'tokens' ? compactTokens(entry.totalTokens) : usd(entry.costMicros);
        this._bar.setParts(masked ? [] : barParts(entry, period));
    }
}

function table(tab, period) {
    return tab === 'projects' ? projectsTable(period) : modelsTable(period);
}

export class SpendBreakdown {
    constructor(ctx, onSelect) {
        this._ctx = ctx;
        this._key = null;
        this._rows = [];
        this.actor = column({ style_class: 'headroom-breakdown', x_expand: true });
        this.actor.add_child(new St.Widget({ style_class: 'headroom-breakdown-separator', x_expand: true }));
        const head = row({ style_class: 'headroom-breakdown-head' });
        this._tabs = new Map();
        const tabs = row({ style_class: 'headroom-segmented mini' });
        for (const [key, title] of TABS) {
            const segment = textButton(title(), 'headroom-segment', () => onSelect(key));
            this._tabs.set(key, segment);
            tabs.add_child(segment);
        }
        head.add_child(tabs);
        head.add_child(spacer());
        this._caption = label('', 'headroom-breakdown-caption');
        head.add_child(this._caption);
        this._list = column({ style_class: 'headroom-breakdown-list', x_expand: true });
        this.actor.add_child(head);
        this.actor.add_child(this._list);
    }

    update(period, tab, unit) {
        const data = table(tab, period);
        this.actor.visible = period.providers.length > 0 && projectsTable(period) !== null && data.rows.length > 0;
        if (!this.actor.visible) return;
        for (const [key, segment] of this._tabs) {
            if (key === tab) segment.add_style_pseudo_class('checked');
            else segment.remove_style_pseudo_class('checked');
        }
        this._caption.text = caption(tab, data.count);
        this._sync(tab, data.rows);
        data.rows.forEach((entry, index) => this._rows[index].update(entry, period, unit));
    }

    _sync(tab, rows) {
        const key = JSON.stringify([tab, this._ctx.masked, rows.map(entry => [entry.name, entry.other])]);
        if (key === this._key) return;
        this._key = key;
        this._list.destroy_all_children();
        this._rows = rows.map(entry => new BreakdownRow(this._ctx, tab, entry));
        for (const breakdownRow of this._rows) this._list.add_child(breakdownRow.actor);
    }
}
