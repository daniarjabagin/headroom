.pragma library

.import "FormatSpend.js" as FormatSpend
.import "I18n.js" as I18n

const PROJECT_CHARS = 26;
const PERMILLE = 1000;
const DASH = "—";

function scaleOf(lang, unit) {
    return {
        lang,
        unit,
        byTokens: unit !== "cost"
    };
}

function weight(entry, byTokens) {
    return byTokens ? entry.totalTokens : entry.costMicros;
}

function permille(value, total) {
    return total > 0 ? Math.floor(value * PERMILLE / total) : 0;
}

function valueText(entry, unit) {
    if (unit === "tokens")
        return FormatSpend.compactTokens(entry.totalTokens);
    if (unit === "cost_per_mtok")
        return entry.costPerMtokMicros === null || entry.costPerMtokMicros === undefined ? DASH : FormatSpend.usd(entry.costPerMtokMicros);
    return FormatSpend.usd(entry.costMicros);
}

function ranking(byTokens, nameOf) {
    return (first, second) => weight(second, byTokens) - weight(first, byTokens) || second.totalTokens - first.totalTokens || nameOf(first).localeCompare(nameOf(second));
}

function segments(parts, total, byTokens) {
    return parts.map(part => ({
                provider: part.provider,
                fraction: total > 0 ? weight(part, byTokens) / total : 0
            })).filter(segment => segment.fraction > 0);
}

function row(scale, key, name, detail, entry, parts, period, share) {
    const total = weight(period, scale.byTokens);
    return {
        key,
        name,
        detail,
        folder: false,
        provider: parts.length === 1 ? parts[0].provider : "",
        value: valueText(entry, scale.unit),
        share: FormatSpend.shareText(scale.lang, share ?? permille(weight(entry, scale.byTokens), total)),
        segments: segments(parts, total, scale.byTokens)
    };
}

function modelCount(lang, count) {
    return I18n.trn(lang, "{count} model", "{count} models", count, {
        count
    });
}

function projectCount(lang, count) {
    return I18n.trn(lang, "{count} project", "{count} projects", count, {
        count
    });
}

function modelRow(scale, model, period) {
    return row(scale, `model:${model.provider}:${model.model}`, model.model, "", model, [model], period);
}

function otherModelRow(scale, key, other, provider, period) {
    const part = Object.assign({}, other, {
        provider
    });
    return row(scale, key, I18n.tr(scale.lang, "Other"), modelCount(scale.lang, other.count), other, [part], period);
}

function mergedModelRows(scale, period) {
    const rows = period.models.map(model => modelRow(scale, model, period));
    if (period.modelsOther !== null)
        rows.push(otherModelRow(scale, "model:other", period.modelsOther, "", period));
    return rows;
}

function providerModelRows(scale, period, spend) {
    const rows = spend.models.map(model => modelRow(scale, Object.assign({}, model, {
                provider: spend.provider
            }), period));
    if (spend.modelsOther !== null)
        rows.push(otherModelRow(scale, `model:other:${spend.provider}`, spend.modelsOther, spend.provider, period));
    return rows;
}

function foldedCount(other) {
    return other?.count ?? 0;
}

function mergedModelCount(period) {
    return period.models.length + foldedCount(period.modelsOther);
}

function providerModelCount(period) {
    return period.providers.reduce((total, spend) => total + spend.models.length + foldedCount(spend.modelsOther), 0);
}

function modelRows(lang, period, unit) {
    const scale = scaleOf(lang, unit);
    const merged = Array.isArray(period.models);
    return {
        rows: merged ? mergedModelRows(scale, period) : period.providers.reduce((rows, spend) => rows.concat(providerModelRows(scale, period, spend)), []),
        caption: modelCount(lang, merged ? mergedModelCount(period) : providerModelCount(period))
    };
}

function projectShare(entry, period, byTokens) {
    return byTokens ? permille(entry.totalTokens, period.totalTokens) : entry.sharePermille;
}

function projectRow(scale, project, period) {
    const name = FormatSpend.projectLabel(scale.lang, project.project, PROJECT_CHARS);
    const result = row(scale, `project:${project.project ?? ""}`, name, "", project, project.providers, period, projectShare(project, period, scale.byTokens));
    result.folder = true;
    return result;
}

function otherProjectRow(scale, other, period) {
    const result = row(scale, "project:other", I18n.tr(scale.lang, "Other"), projectCount(scale.lang, other.count), other, [Object.assign({
            provider: ""
        }, other)], period, projectShare(other, period, scale.byTokens));
    result.folder = true;
    return result;
}

function projectsByRank(period, byTokens) {
    return Array.from(period.projects).sort(ranking(byTokens, project => project.project ?? ""));
}

function projectRows(lang, period, unit) {
    const scale = scaleOf(lang, unit);
    const rows = projectsByRank(period, scale.byTokens).map(project => projectRow(scale, project, period));
    const other = period.projectsOther;
    if (other !== null)
        rows.push(otherProjectRow(scale, other, period));
    return {
        rows,
        caption: projectCount(lang, period.projects.length + (other?.count ?? 0))
    };
}

function hasProjects(period) {
    return (period?.projects?.length ?? 0) > 0;
}

function breakdown(lang, period, mode, unit) {
    if (!hasProjects(period))
        return null;
    return mode === "projects" ? projectRows(lang, period, unit) : modelRows(lang, period, unit);
}
