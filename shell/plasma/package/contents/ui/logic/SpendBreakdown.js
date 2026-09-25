.pragma library

.import "FormatSpend.js" as FormatSpend
.import "I18n.js" as I18n

const TOP_MODELS = 5;
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

function modelsOf(period, byTokens) {
    const listed = [];
    const folded = [];
    for (const spend of period.providers) {
        for (const model of spend.models)
            listed.push({
                name: model.model,
                provider: spend.provider,
                costMicros: model.costMicros,
                totalTokens: model.totalTokens,
                costPerMtokMicros: model.costPerMtokMicros ?? null,
                count: 1
            });
        if (spend.modelsOther !== null)
            folded.push({
                provider: spend.provider,
                costMicros: spend.modelsOther.costMicros,
                totalTokens: spend.modelsOther.totalTokens,
                count: spend.modelsOther.count
            });
    }
    listed.sort(ranking(byTokens, model => model.name));
    return {
        top: listed.slice(0, TOP_MODELS),
        folded: folded.concat(listed.slice(TOP_MODELS))
    };
}

function mergedByProvider(entries) {
    const merged = [];
    for (const entry of entries) {
        const found = merged.find(candidate => candidate.provider === entry.provider);
        if (found) {
            found.costMicros += entry.costMicros;
            found.totalTokens += entry.totalTokens;
        } else
            merged.push({
                provider: entry.provider,
                costMicros: entry.costMicros,
                totalTokens: entry.totalTokens
            });
    }
    return merged;
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

function sum(entries) {
    return entries.reduce((total, entry) => ({
                costMicros: total.costMicros + entry.costMicros,
                totalTokens: total.totalTokens + entry.totalTokens,
                costPerMtokMicros: null,
                count: total.count + entry.count
            }), {
        costMicros: 0,
        totalTokens: 0,
        costPerMtokMicros: null,
        count: 0
    });
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

function modelRows(lang, period, unit) {
    const scale = scaleOf(lang, unit);
    const models = modelsOf(period, scale.byTokens);
    const rows = models.top.map(model => row(scale, `model:${model.provider}:${model.name}`, model.name, "", model, [model], period));
    const other = sum(models.folded);
    if (other.count > 0)
        rows.push(row(scale, "model:other", I18n.tr(lang, "Other"), modelCount(lang, other.count), other, mergedByProvider(models.folded), period));
    return {
        rows,
        caption: modelCount(lang, models.top.length + other.count)
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
