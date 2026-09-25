.pragma library

.import "FormatSpend.js" as FormatSpend
.import "I18n.js" as I18n

function costText(lang, model) {
    if (model.partial && model.costMicros === 0)
        return I18n.tr(lang, "unpriced");
    return FormatSpend.usd(model.costMicros);
}

function modelRow(lang, model) {
    return {
        name: model.model,
        tokens: FormatSpend.compactTokens(model.totalTokens),
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

function shareOf(model, totals) {
    if (totals.costMicros > 0)
        return model.costMicros / totals.costMicros;
    return totals.totalTokens > 0 ? model.totalTokens / totals.totalTokens : 0;
}

function popoverRow(lang, model, totals, name, detail) {
    const share = shareOf(model, totals);
    return {
        name,
        detail,
        cost: costText(lang, model),
        fraction: share,
        partial: model.partial,
        figures: I18n.tr(lang, "{percent}% · {tokens} tokens", {
            percent: Math.floor(share * 100),
            tokens: FormatSpend.compactTokens(model.totalTokens)
        })
    };
}

function popoverRows(lang, totals) {
    const models = Array.from(totals?.models ?? []);
    const rows = models.map(model => popoverRow(lang, model, totals, model.model, ""));
    if (rows.length === 0)
        return [];
    const other = totals.modelsOther;
    if (other)
        rows.push(popoverRow(lang, other, totals, I18n.tr(lang, "Other"), I18n.trn(lang, "{count} model", "{count} models", other.count, {
            count: other.count
        })));
    return rows;
}

function popoverNotes(lang, totals) {
    const notes = [];
    if (totals.modelsOther)
        notes.push(I18n.tr(lang, "Models after the top five are folded into Other."));
    notes.push(I18n.tr(lang, "Estimated from local logs and public pricing."));
    if (totals.partial)
        notes.push(I18n.tr(lang, "Partly unpriced, cost leaves it out"));
    return notes;
}

function totalLine(lang, totals) {
    return I18n.tr(lang, "{cost} · {tokens}", {
        cost: FormatSpend.exactUsd(totals.costMicros),
        tokens: FormatSpend.tokenCount(lang, totals.totalTokens)
    });
}
