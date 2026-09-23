.pragma library

.import "Format.js" as Format
.import "I18n.js" as I18n

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

function otherRow(lang, other) {
    return modelRow(lang, {
        model: I18n.tr(lang, "Other ({count})", {
            count: other.count
        }),
        totalTokens: other.totalTokens,
        costMicros: other.costMicros,
        partial: other.partial
    });
}

function modelBreakdown(lang, models, other) {
    if (!Array.isArray(models) || models.length === 0)
        return null;
    const rows = models.map(model => modelRow(lang, model));
    if (other)
        rows.push(otherRow(lang, other));
    return {
        rows,
        partial: models.some(model => model.partial) || (other?.partial ?? false)
    };
}

function totalLine(lang, totals) {
    return I18n.tr(lang, "{cost} · {tokens}", {
        cost: Format.exactUsd(totals.costMicros),
        tokens: Format.tokenCount(lang, totals.totalTokens)
    });
}
