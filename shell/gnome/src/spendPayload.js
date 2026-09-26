import { count, integer, isObject, list, providerOf, text } from './fields.js';

function costPerMtok(raw) {
    const micros = integer(raw.cost_per_mtok_usd_micros);
    return micros !== null && micros >= 0 ? micros : null;
}

export function parseTokens(raw) {
    const tokens = isObject(raw) ? raw : {};
    return {
        input: count(tokens.input),
        cacheRead: count(tokens.cache_read),
        cacheWrite: count(tokens.cache_write),
        output: count(tokens.output),
        reasoning: count(tokens.reasoning),
        total: count(tokens.total),
    };
}

function parseModel(raw) {
    return {
        model: text(raw.model) ?? 'unknown',
        totalTokens: count(raw.total_tokens),
        costMicros: count(raw.cost_usd_micros),
        partial: raw.partial === true,
        costPerMtokMicros: costPerMtok(raw),
    };
}

function parseModels(raw) {
    return list(raw).map(parseModel);
}

function parseModelsOther(raw) {
    if (!isObject(raw)) return null;
    return {
        count: count(raw.count),
        totalTokens: count(raw.total_tokens),
        costMicros: count(raw.cost_usd_micros),
        partial: raw.partial === true,
        costPerMtokMicros: costPerMtok(raw),
    };
}

export function parseTotals(raw) {
    const totals = isObject(raw) ? raw : {};
    const tokens = parseTokens(totals.tokens);
    return {
        tokens,
        costMicros: count(totals.cost_usd_micros),
        totalTokens: tokens.total,
        partial: totals.partial === true,
        unpricedTokens: count(totals.unpriced_tokens),
        unpricedModels: Array.isArray(totals.unpriced_models) ? totals.unpriced_models.filter(text) : [],
        models: parseModels(totals.models),
        modelsOther: parseModelsOther(totals.models_other),
    };
}

function parseDay(raw) {
    return {
        date: text(raw.date) ?? '',
        totalTokens: count(raw.total_tokens),
        costMicros: count(raw.cost_usd_micros),
        partial: raw.partial === true,
    };
}

export function parseUsage(raw) {
    return {
        ...providerOf(raw),
        usageHome: text(raw.usage_home),
        today: parseTotals(raw.today),
        yesterday: parseTotals(raw.yesterday),
        last30Days: parseTotals(raw.last_30_days),
        daily: list(raw.daily).map(parseDay),
    };
}

function parseProviderSpend(raw) {
    return {
        ...providerOf(raw),
        costMicros: count(raw.cost_usd_micros),
        totalTokens: count(raw.total_tokens),
        partial: raw.partial === true,
        costPerMtokMicros: costPerMtok(raw),
        models: parseModels(raw.models),
        modelsOther: parseModelsOther(raw.models_other),
    };
}

function parseProjectProvider(raw) {
    return { ...providerOf(raw), costMicros: count(raw.cost_usd_micros), totalTokens: count(raw.total_tokens) };
}

function projectShare(raw) {
    return {
        costMicros: count(raw.cost_usd_micros),
        totalTokens: count(raw.total_tokens),
        partial: raw.partial === true,
        sharePermille: integer(raw.share_permille) ?? 0,
        costPerMtokMicros: costPerMtok(raw),
    };
}

function parseProject(raw) {
    return {
        project: text(raw.project),
        ...projectShare(raw),
        providers: list(raw.by_provider).map(parseProjectProvider),
    };
}

function parseProjects(raw) {
    return Array.isArray(raw) ? list(raw).map(parseProject) : null;
}

function parseProjectsOther(raw) {
    return isObject(raw) ? { count: count(raw.count), ...projectShare(raw) } : null;
}

function parsePeriod(raw) {
    const period = isObject(raw) ? raw : {};
    return {
        costMicros: count(period.cost_usd_micros),
        totalTokens: count(period.total_tokens),
        partial: period.partial === true,
        costPerMtokMicros: costPerMtok(period),
        providers: list(period.by_provider).map(parseProviderSpend),
        projects: parseProjects(period.projects),
        projectsOther: parseProjectsOther(period.projects_other),
    };
}

export function parseSpend(raw, usage) {
    if (usage.length === 0 || !isObject(raw)) return null;
    return {
        today: parsePeriod(raw.today),
        yesterday: parsePeriod(raw.yesterday),
        last7Days: isObject(raw.last_7_days) ? parsePeriod(raw.last_7_days) : null,
        last30Days: parsePeriod(raw.last_30_days),
    };
}
