import { compactTokens, exactTokens, usd } from './format.js';
import { _, fill } from './i18n.js';

export const TOP_MODELS = 5;

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

function otherRow(models) {
    return modelRow({
        model: fill(_('Other ({count})'), { count: models.length }),
        totalTokens: models.reduce((sum, model) => sum + model.totalTokens, 0),
        costMicros: models.reduce((sum, model) => sum + model.costMicros, 0),
        partial: models.some(model => model.partial),
    });
}

export function modelBreakdown(models, limit = TOP_MODELS) {
    if (models.length === 0) return null;
    const shown = models.length > limit + 1 ? models.slice(0, limit) : models;
    const rest = models.slice(shown.length);
    const rows = shown.map(modelRow);
    if (rest.length > 0) rows.push(otherRow(rest));
    return { rows, partial: models.some(model => model.partial) };
}
