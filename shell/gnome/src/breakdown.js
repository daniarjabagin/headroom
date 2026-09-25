import { _, currentLanguage, fill, n_ } from './i18n.js';
import { compactTokens, compactTokensText, exactTokens, usd } from './numbers.js';
import { seriesKey } from './providers.js';

const PERMILLE = 1000;
const TOP_MODELS = 5;
const MICROS_PER_MTOK = 1_000_000n;
const DASH = '—';

function costText(model) {
    if (model.partial && model.costMicros === 0) return _('unpriced');
    return usd(model.costMicros);
}

export function sharePermille(part, total) {
    if (total <= 0 || part <= 0) return 0;
    return Math.floor((part * PERMILLE) / total);
}

function shareOf(entry, totals) {
    if (totals.costMicros > 0) return sharePermille(entry.costMicros, totals.costMicros);
    return sharePermille(entry.totalTokens, totals.totalTokens);
}

export function wholeShareText(permille) {
    return `${Math.floor(permille / 10)}%`;
}

export function preciseShareText(permille) {
    const text = `${Math.floor(permille / 10)}.${permille % 10}%`;
    return currentLanguage() === 'ru' ? text.replace('.', ',') : text;
}

export function otherModelsText(count) {
    return fill(n_('· {count} model', '· {count} models', count), { count });
}

export function otherProjectsText(count) {
    return fill(n_('· {count} project', '· {count} projects', count), { count });
}

function modelRow(model, totals) {
    return {
        name: model.model,
        detail: null,
        tokens: compactTokens(model.totalTokens),
        tokensText: compactTokensText(model.totalTokens),
        exactTokens: exactTokens(model.totalTokens),
        cost: costText(model),
        partial: model.partial,
        sharePermille: totals ? shareOf(model, totals) : 0,
    };
}

function otherRow(other, totals) {
    return {
        ...modelRow({ ...other, model: _('Other') }, totals),
        detail: otherModelsText(other.count),
    };
}

export function modelBreakdown(models, other, totals = null) {
    if (models.length === 0 && other === null) return null;
    const rows = models.map(model => modelRow(model, totals));
    if (other !== null) rows.push(otherRow(other, totals));
    return { rows, partial: rows.some(entry => entry.partial) };
}

export function shareMeasure(period, unit) {
    return unit === 'cost' && period.costMicros > 0 ? 'costMicros' : 'totalTokens';
}

export function perMtokMicros(costMicros, totalTokens, partial) {
    if (partial || totalTokens <= 0 || costMicros < 0) return null;
    const tokens = BigInt(totalTokens);
    return Number((BigInt(costMicros) * MICROS_PER_MTOK * 2n + tokens) / (tokens * 2n));
}

function byMeasure(measure) {
    return (a, b) =>
        b[measure] - a[measure] || b.totalTokens - a.totalTokens || (a.name ?? '').localeCompare(b.name ?? '');
}

function flatModels(period) {
    return period.providers.flatMap(spend =>
        spend.models.map(model => ({
            name: model.model,
            costMicros: model.costMicros,
            totalTokens: model.totalTokens,
            partial: model.partial,
            costPerMtokMicros: model.costPerMtokMicros ?? null,
            parts: [
                { series: seriesKey(spend.provider), costMicros: model.costMicros, totalTokens: model.totalTokens },
            ],
        }))
    );
}

function providerOthers(period) {
    return period.providers
        .filter(spend => spend.modelsOther !== null)
        .map(spend => ({ series: seriesKey(spend.provider), ...spend.modelsOther }));
}

function addPart(parts, part) {
    const found = parts.find(entry => entry.series === part.series);
    if (!found) return [...parts, { series: part.series, costMicros: part.costMicros, totalTokens: part.totalTokens }];
    return parts.map(entry =>
        entry === found
            ? {
                  ...entry,
                  costMicros: entry.costMicros + part.costMicros,
                  totalTokens: entry.totalTokens + part.totalTokens,
              }
            : entry
    );
}

function foldedModels(rest, others) {
    const pieces = [...rest.map(model => ({ ...model.parts[0], count: 1, partial: model.partial })), ...others];
    if (pieces.length === 0) return null;
    const costMicros = pieces.reduce((sum, piece) => sum + piece.costMicros, 0);
    const totalTokens = pieces.reduce((sum, piece) => sum + piece.totalTokens, 0);
    const partial = pieces.some(piece => piece.partial);
    return {
        count: pieces.reduce((sum, piece) => sum + piece.count, 0),
        costMicros,
        totalTokens,
        partial,
        costPerMtokMicros: perMtokMicros(costMicros, totalTokens, partial),
        parts: pieces.reduce(addPart, []),
    };
}

export function modelsTable(period, unit) {
    const measure = shareMeasure(period, unit);
    const shareOfPeriod = entry => sharePermille(entry[measure], period[measure]);
    const sorted = flatModels(period).sort(byMeasure(measure));
    const other = foldedModels(sorted.slice(TOP_MODELS), providerOthers(period));
    const listed = sorted.slice(0, TOP_MODELS);
    const rows = listed.map(model => ({ ...model, sharePermille: shareOfPeriod(model), other: null }));
    if (other) rows.push({ ...other, name: _('Other'), sharePermille: shareOfPeriod(other), other: other.count });
    return { rows, count: listed.length + (other?.count ?? 0) };
}

function projectParts(project) {
    return project.providers.map(spend => ({
        series: seriesKey(spend.provider),
        costMicros: spend.costMicros,
        totalTokens: spend.totalTokens,
    }));
}

function projectFigures(entry, shareOf) {
    return {
        costMicros: entry.costMicros,
        totalTokens: entry.totalTokens,
        costPerMtokMicros: entry.costPerMtokMicros ?? null,
        sharePermille: shareOf(entry),
    };
}

function projectShares(period, measure) {
    if (measure === 'costMicros') return entry => entry.sharePermille;
    return entry => sharePermille(entry.totalTokens, period.totalTokens);
}

export function projectsTable(period, unit) {
    if (!Array.isArray(period.projects)) return null;
    const measure = shareMeasure(period, unit);
    const shareOf = projectShares(period, measure);
    const rows = period.projects.map(project => ({
        name: project.project,
        ...projectFigures(project, shareOf),
        parts: projectParts(project),
        other: null,
    }));
    if (measure === 'totalTokens') rows.sort(byMeasure(measure));
    const other = period.projectsOther;
    if (other) rows.push({ ...projectFigures(other, shareOf), parts: [], name: _('Other'), other: other.count });
    return { rows, count: period.projects.length + (other?.count ?? 0) };
}

export function barParts(row, period, unit) {
    const measure = shareMeasure(period, unit);
    const total = period[measure];
    if (row.parts.length === 0) return [{ series: null, permille: row.sharePermille }];
    return row.parts.map(part => ({ series: part.series, permille: sharePermille(part[measure], total) }));
}

export function breakdownValue(entry, unit) {
    if (unit === 'tokens') return compactTokens(entry.totalTokens);
    if (unit === 'cost_per_mtok') return entry.costPerMtokMicros === null ? DASH : usd(entry.costPerMtokMicros);
    return usd(entry.costMicros);
}
