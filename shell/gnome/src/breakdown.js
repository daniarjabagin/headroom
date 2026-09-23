import { _, fill } from './i18n.js';
import { compactTokens, exactTokens, usd } from './numbers.js';

function costText(model) {
    if (model.partial && model.costMicros === 0) return _('unpriced');
    return usd(model.costMicros);
}

function modelRow(model) {
    return {
        name: model.model,
        tokens: compactTokens(model.totalTokens),
        exactTokens: exactTokens(model.totalTokens),
        cost: costText(model),
        partial: model.partial,
    };
}

function otherRow(other) {
    return modelRow({
        model: fill(_('Other ({count})'), { count: other.count }),
        totalTokens: other.totalTokens,
        costMicros: other.costMicros,
        partial: other.partial,
    });
}

export function modelBreakdown(models, other) {
    if (models.length === 0 && other === null) return null;
    const rows = models.map(modelRow);
    if (other !== null) rows.push(otherRow(other));
    return { rows, partial: rows.some(entry => entry.partial) };
}
