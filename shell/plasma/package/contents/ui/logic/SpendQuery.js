.pragma library

.import "Parse.js" as Parse
.import "SpendState.js" as SpendState

const GROUPS = ["model", "project", "provider", "day"];
const PERIODS = ["today", "yesterday", "7d", "30d"];
const DATE = /^\d{4}-\d\d-\d\d$/;

function date(value) {
    return typeof value === "string" && DATE.test(value) ? value : null;
}

function withProvider(query, provider) {
    const id = Parse.text(provider);
    return id === null ? query : Object.assign(query, {
        provider: id
    });
}

function periodQuery(period, by, provider) {
    if (!PERIODS.includes(period) || !GROUPS.includes(by))
        return null;
    return withProvider({
        period,
        by
    }, provider);
}

function rangeQuery(since, until, by, provider) {
    const first = date(since);
    const last = until === null ? null : date(until);
    if (first === null || !GROUPS.includes(by) || (until !== null && (last === null || last < first)))
        return null;
    const query = last === null ? {
        since: first,
        by
    } : {
        since: first,
        until: last,
        by
    };
    return withProvider(query, provider);
}

function parseRow(raw) {
    return {
        key: Parse.text(raw.key),
        provider: Parse.text(raw.provider),
        tokens: SpendState.parseTokens(raw.tokens),
        costMicros: Parse.count(raw.cost_usd_micros),
        partial: raw.partial === true,
        unpricedTokens: Parse.count(raw.unpriced_tokens),
        costPerMtokMicros: Parse.integer(raw.cost_per_mtok_usd_micros),
        sharePermille: Parse.count(raw.share_permille)
    };
}

function decode(json) {
    try {
        return JSON.parse(json);
    } catch (error) {
        return null;
    }
}

function parseResult(json) {
    const raw = decode(json);
    if (!Parse.isObject(raw) || !GROUPS.includes(raw.by) || date(raw.since) === null || date(raw.until) === null || !Parse.isObject(raw.total))
        return null;
    return {
        since: raw.since,
        until: raw.until,
        by: raw.by,
        rows: Parse.list(raw.rows).map(parseRow),
        total: parseRow(raw.total)
    };
}
