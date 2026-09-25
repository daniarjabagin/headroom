.pragma library

.import "Parse.js" as Parse

function providerName(raw, provider) {
    return Parse.text(raw.provider_name) ?? provider;
}

function parseTokens(raw) {
    const tokens = Parse.object(raw);
    return {
        input: Parse.count(tokens.input),
        cacheRead: Parse.count(tokens.cache_read),
        cacheWrite: Parse.count(tokens.cache_write),
        output: Parse.count(tokens.output),
        reasoning: Parse.count(tokens.reasoning),
        total: Parse.count(tokens.total)
    };
}

function parseModel(raw) {
    return {
        model: Parse.text(raw.model) ?? "unknown",
        totalTokens: Parse.count(raw.total_tokens),
        costMicros: Parse.count(raw.cost_usd_micros),
        costPerMtokMicros: Parse.integer(raw.cost_per_mtok_usd_micros),
        partial: raw.partial === true
    };
}

function parseModels(raw) {
    return Parse.list(raw).map(parseModel);
}

function parseModelsOther(raw) {
    if (!Parse.isObject(raw) || Parse.count(raw.count) === 0)
        return null;
    return {
        count: Parse.count(raw.count),
        totalTokens: Parse.count(raw.total_tokens),
        costMicros: Parse.count(raw.cost_usd_micros),
        partial: raw.partial === true
    };
}

function parseTotals(raw) {
    const totals = Parse.object(raw);
    const tokens = parseTokens(totals.tokens);
    return {
        tokens,
        costMicros: Parse.count(totals.cost_usd_micros),
        totalTokens: tokens.total,
        partial: totals.partial === true,
        unpricedTokens: Parse.count(totals.unpriced_tokens),
        unpricedModels: Array.isArray(totals.unpriced_models) ? totals.unpriced_models.filter(Parse.text) : [],
        models: parseModels(totals.models),
        modelsOther: parseModelsOther(totals.models_other)
    };
}

function parseDay(raw) {
    return {
        date: Parse.text(raw.date) ?? "",
        totalTokens: Parse.count(raw.total_tokens),
        costMicros: Parse.count(raw.cost_usd_micros),
        partial: raw.partial === true
    };
}

function parseUsage(raw) {
    const provider = Parse.text(raw.provider) ?? "unknown";
    return {
        provider,
        providerName: providerName(raw, provider),
        usageHome: Parse.text(raw.usage_home),
        today: parseTotals(raw.today),
        yesterday: parseTotals(raw.yesterday),
        last30Days: parseTotals(raw.last_30_days),
        daily: Parse.list(raw.daily).map(parseDay)
    };
}

function parseProviderSpend(raw) {
    const provider = Parse.text(raw.provider) ?? "unknown";
    return {
        provider,
        providerName: providerName(raw, provider),
        costMicros: Parse.count(raw.cost_usd_micros),
        totalTokens: Parse.count(raw.total_tokens),
        costPerMtokMicros: Parse.integer(raw.cost_per_mtok_usd_micros),
        partial: raw.partial === true,
        models: parseModels(raw.models),
        modelsOther: parseModelsOther(raw.models_other)
    };
}

function parseProjectProvider(raw) {
    const provider = Parse.text(raw.provider) ?? "unknown";
    return {
        provider,
        providerName: providerName(raw, provider),
        costMicros: Parse.count(raw.cost_usd_micros),
        totalTokens: Parse.count(raw.total_tokens)
    };
}

function parseProject(raw) {
    return {
        project: Parse.text(raw.project),
        costMicros: Parse.count(raw.cost_usd_micros),
        totalTokens: Parse.count(raw.total_tokens),
        partial: raw.partial === true,
        sharePermille: Parse.count(raw.share_permille),
        providers: Parse.list(raw.by_provider).map(parseProjectProvider)
    };
}

function parseProjectsOther(raw) {
    if (!Parse.isObject(raw) || Parse.count(raw.count) === 0)
        return null;
    return {
        count: Parse.count(raw.count),
        costMicros: Parse.count(raw.cost_usd_micros),
        totalTokens: Parse.count(raw.total_tokens),
        partial: raw.partial === true,
        sharePermille: Parse.count(raw.share_permille)
    };
}

function parsePeriod(raw) {
    const period = Parse.object(raw);
    return {
        costMicros: Parse.count(period.cost_usd_micros),
        totalTokens: Parse.count(period.total_tokens),
        costPerMtokMicros: Parse.integer(period.cost_per_mtok_usd_micros),
        partial: period.partial === true,
        providers: Parse.list(period.by_provider).map(parseProviderSpend),
        projects: Parse.optionalList(period.projects, parseProject),
        projectsOther: parseProjectsOther(period.projects_other)
    };
}

function parseOptionalPeriod(raw) {
    return Parse.isObject(raw) ? parsePeriod(raw) : null;
}

function parseSpend(raw, usage) {
    if (usage.length === 0 || !Parse.isObject(raw))
        return null;
    return {
        today: parsePeriod(raw.today),
        yesterday: parsePeriod(raw.yesterday),
        last7Days: parseOptionalPeriod(raw.last_7_days),
        last30Days: parsePeriod(raw.last_30_days)
    };
}
