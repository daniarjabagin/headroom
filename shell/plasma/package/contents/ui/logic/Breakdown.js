.pragma library

.import "Format.js" as Format
.import "I18n.js" as I18n

const TOP_MODELS = 5;

function costText(lang, model) {
    if (model.partial && model.costMicros === 0)
        return I18n.tr(lang, "unpriced");
    return Format.usd(model.costMicros);
}

function modelRow(lang, model) {
    return {
        name: model.model,
        tokens: Format.compactTokens(model.totalTokens),
        cost: costText(lang, model),
        partial: model.partial
    };
}

function otherRow(lang, models) {
    return modelRow(lang, {
        model: I18n.tr(lang, "Other ({count})", {
            count: models.length
        }),
        totalTokens: models.reduce((sum, model) => sum + model.totalTokens, 0),
        costMicros: models.reduce((sum, model) => sum + model.costMicros, 0),
        partial: models.some(model => model.partial)
    });
}

function modelBreakdown(lang, models, limit) {
    const top = limit ?? TOP_MODELS;
    if (!Array.isArray(models) || models.length === 0)
        return null;
    const shown = models.length > top + 1 ? models.slice(0, top) : models;
    const rest = models.slice(shown.length);
    const rows = shown.map(model => modelRow(lang, model));
    if (rest.length > 0)
        rows.push(otherRow(lang, rest));
    return {
        rows,
        partial: models.some(model => model.partial)
    };
}

function totalLine(lang, totals) {
    return I18n.tr(lang, "{cost} · {tokens}", {
        cost: Format.exactUsd(totals.costMicros),
        tokens: Format.tokenCount(lang, totals.totalTokens)
    });
}
