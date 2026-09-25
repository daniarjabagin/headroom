import { count, integer, isObject, list, text } from './fields.js';
import { parseTokens } from './spendPayload.js';

export const SPEND_GROUPINGS = ['model', 'project', 'provider', 'day'];
export const QUERY_PERIODS = ['today', 'yesterday', '7d', '30d'];

const ISO_DATE = /^\d{4}-\d{2}-\d{2}$/;

export class SpendQueryError extends Error {}

function isDate(value) {
    return typeof value === 'string' && ISO_DATE.test(value);
}

function range({ period, since, until }) {
    if (period !== undefined && since !== undefined)
        throw new SpendQueryError('Give a period or a start date, not both');
    if (period !== undefined) {
        if (!QUERY_PERIODS.includes(period)) throw new SpendQueryError(`Unknown spend period ${period}`);
        if (until !== undefined) throw new SpendQueryError('An end date needs a start date');
        return { period };
    }
    if (!isDate(since)) throw new SpendQueryError('A spend query needs a period or a start date');
    if (until === undefined) return { since };
    if (!isDate(until) || until < since) throw new SpendQueryError('The end date must not be before the start date');
    return { since, until };
}

export function spendQuery(options) {
    if (!SPEND_GROUPINGS.includes(options.by)) throw new SpendQueryError(`Unknown spend grouping ${options.by}`);
    const query = { ...range(options), by: options.by };
    if (options.provider === undefined) return query;
    if (!text(options.provider)) throw new SpendQueryError('Provider id must not be empty');
    return { ...query, provider: options.provider };
}

function parseRow(raw) {
    const row = isObject(raw) ? raw : {};
    const tokens = parseTokens(row.tokens);
    return {
        key: text(row.key),
        provider: text(row.provider),
        tokens,
        totalTokens: tokens.total,
        costMicros: count(row.cost_usd_micros),
        partial: row.partial === true,
        unpricedTokens: count(row.unpriced_tokens),
        costPerMtokMicros: integer(row.cost_per_mtok_usd_micros),
        sharePermille: integer(row.share_permille) ?? 0,
    };
}

function decode(json) {
    try {
        return JSON.parse(json);
    } catch (error) {
        throw new SpendQueryError(`Unreadable spend report from the Headroom service: ${error.message}`);
    }
}

export function parseSpendResult(json) {
    const raw = decode(json);
    if (!isObject(raw) || !SPEND_GROUPINGS.includes(raw.by))
        throw new SpendQueryError('Unexpected spend report from the Headroom service');
    return {
        since: text(raw.since),
        until: text(raw.until),
        by: raw.by,
        rows: list(raw.rows).map(parseRow),
        total: parseRow(raw.total),
    };
}
