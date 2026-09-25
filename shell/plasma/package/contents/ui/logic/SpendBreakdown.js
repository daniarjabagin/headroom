.pragma library

.import "FormatSpend.js" as FormatSpend
.import "I18n.js" as I18n

const TOP_MODELS = 5;
const PROJECT_CHARS = 26;
const PERMILLE = 1000;

function weight(entry, byTokens) {
    return byTokens ? entry.totalTokens : entry.costMicros;
}

function permille(value, total) {
    return total > 0 ? Math.floor(value * PERMILLE / total) : 0;
}

function shareText(value) {
    return `${(value / 10).toFixed(1)}%`;
}

function valueText(entry, byTokens) {
    return byTokens ? FormatSpend.compactTokens(entry.totalTokens) : FormatSpend.usd(entry.costMicros);
}

function byRank(first, second) {
    return second.costMicros - first.costMicros || second.totalTokens - first.totalTokens || first.name.localeCompare(second.name);
}

function modelsOf(period) {
    const listed = [];
    const folded = [];
    for (const spend of period.providers) {
        for (const model of spend.models)
            listed.push({
                name: model.model,
                provider: spend.provider,
                costMicros: model.costMicros,
                totalTokens: model.totalTokens,
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
    listed.sort(byRank);
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

function row(key, name, detail, entry, parts, period, byTokens, share) {
    const total = weight(period, byTokens);
    return {
        key,
        name,
        detail,
        folder: false,
        provider: parts.length === 1 ? parts[0].provider : "",
        value: valueText(entry, byTokens),
        share: shareText(share ?? permille(weight(entry, byTokens), total)),
        segments: segments(parts, total, byTokens)
    };
}

function sum(entries) {
    return entries.reduce((total, entry) => ({
                costMicros: total.costMicros + entry.costMicros,
                totalTokens: total.totalTokens + entry.totalTokens,
                count: total.count + entry.count
            }), {
        costMicros: 0,
        totalTokens: 0,
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

function modelRows(lang, period, byTokens) {
    const models = modelsOf(period);
    const rows = models.top.map(model => row(`model:${model.provider}:${model.name}`, model.name, "", model, [model], period, byTokens));
    const other = sum(models.folded);
    if (other.count > 0)
        rows.push(row("model:other", I18n.tr(lang, "Other"), modelCount(lang, other.count), other, mergedByProvider(models.folded), period, byTokens));
    return {
        rows,
        caption: modelCount(lang, models.top.length + other.count)
    };
}

function projectShare(entry, period, byTokens) {
    return byTokens ? permille(entry.totalTokens, period.totalTokens) : entry.sharePermille;
}

function projectRow(lang, project, period, byTokens) {
    const name = FormatSpend.projectLabel(lang, project.project, PROJECT_CHARS);
    const result = row(`project:${project.project ?? ""}`, name, "", project, project.providers, period, byTokens, projectShare(project, period, byTokens));
    result.folder = true;
    return result;
}

function otherProjectRow(lang, other, period, byTokens) {
    const result = row("project:other", I18n.tr(lang, "Other"), projectCount(lang, other.count), other, [Object.assign({
            provider: ""
        }, other)], period, byTokens, projectShare(other, period, byTokens));
    result.folder = true;
    return result;
}

function projectRows(lang, period, byTokens) {
    const rows = Array.from(period.projects).map(project => projectRow(lang, project, period, byTokens));
    const other = period.projectsOther;
    if (other !== null)
        rows.push(otherProjectRow(lang, other, period, byTokens));
    return {
        rows,
        caption: projectCount(lang, period.projects.length + (other?.count ?? 0))
    };
}

function hasProjects(period) {
    return (period?.projects?.length ?? 0) > 0;
}

function breakdown(lang, period, mode, byTokens) {
    if (!hasProjects(period))
        return null;
    return mode === "projects" ? projectRows(lang, period, byTokens) : modelRows(lang, period, byTokens);
}
