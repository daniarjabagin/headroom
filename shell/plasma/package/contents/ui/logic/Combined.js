.pragma library

.import "Format.js" as Format
.import "I18n.js" as I18n
.import "Providers.js" as Providers
.import "Quota.js" as Quota

const DETAIL_SEPARATOR = " · ";
const DEFAULT_CAPACITY = 100;

function isObject(value) {
    return value !== null && typeof value === "object" && !Array.isArray(value);
}

function text(value) {
    return typeof value === "string" && value.length > 0 ? value : null;
}

function objects(value) {
    return Array.isArray(value) ? value.filter(isObject) : [];
}

function parseSegment(raw, parseWindow) {
    return Object.assign(parseWindow(raw), {
        accountId: text(raw.account_id) ?? "",
        label: text(raw.label)
    });
}

function parseCombinedWindow(raw, parseWindow) {
    const capacity = raw.capacity_percent;
    return Object.assign(parseWindow(raw), {
        capacityPercent: typeof capacity === "number" && Number.isFinite(capacity) ? capacity : DEFAULT_CAPACITY,
        segments: objects(raw.segments).map(segment => parseSegment(segment, parseWindow))
    });
}

function parseMember(raw) {
    return {
        accountId: text(raw.account_id) ?? "",
        label: text(raw.label),
        plan: text(raw.plan)
    };
}

function parseGroup(raw, parseWindow) {
    const provider = text(raw.provider) ?? "unknown";
    return {
        provider,
        providerName: text(raw.provider_name) ?? provider,
        accountIds: Array.isArray(raw.account_ids) ? raw.account_ids.filter(text) : [],
        accounts: objects(raw.accounts).map(parseMember),
        windows: objects(raw.windows).map(window => parseCombinedWindow(window, parseWindow))
    };
}

function parseGroups(raw, parseWindow) {
    return raw.map(group => parseGroup(group, parseWindow)).filter(group => group.accountIds.length > 0);
}

function accountCard(account) {
    return {
        kind: "account",
        id: account.id,
        accountIds: [account.id],
        account,
        group: null,
        members: []
    };
}

function combinedCard(group, accounts) {
    const members = accounts.filter(account => group.accountIds.includes(account.id));
    return {
        kind: "combined",
        id: `combined:${group.provider}:${group.accountIds.join(",")}`,
        accountIds: members.map(account => account.id),
        account: null,
        group,
        members
    };
}

function cards(state, accounts) {
    const groups = state.display.combineAccounts ? state.combined : [];
    const placed = [];
    const found = [];
    for (const account of accounts) {
        const group = groups.find(candidate => candidate.accountIds.includes(account.id));
        if (!group)
            found.push(accountCard(account));
        else if (!placed.includes(group)) {
            placed.push(group);
            found.push(combinedCard(group, accounts));
        }
    }
    return found;
}

function accountCountText(lang, count) {
    return I18n.trn(lang, "{count} account", "{count} accounts", count, {
        count
    });
}

function groupDetail(lang, group) {
    const plans = group.accounts.map(member => member.plan).filter((plan, index, all) => plan !== null && all.indexOf(plan) === index);
    return [accountCountText(lang, group.accountIds.length)].concat(plans).join(DETAIL_SEPARATOR);
}

function groupStatus(members) {
    if (members.some(account => account.status === "refreshing"))
        return "refreshing";
    if (members.some(account => account.status === "stale" || account.status === "error"))
        return "stale";
    return "fresh";
}

function oldestUpdate(members) {
    const stamps = members.map(account => account.updatedAt).filter(stamp => stamp !== null);
    return stamps.length === 0 ? null : new Date(Math.min(...stamps));
}

function headerAccount(lang, card) {
    return {
        id: card.id,
        provider: card.group.provider,
        providerName: card.group.providerName,
        label: null,
        email: null,
        plan: groupDetail(lang, card.group),
        status: groupStatus(card.members),
        error: null,
        updatedAt: oldestUpdate(card.members),
        windows: [],
        balances: [],
        notices: [],
        usage: null
    };
}

function combinedPercent(window, valueMode) {
    if (window.remainingPercent === null)
        return null;
    if (valueMode === "used")
        return window.usedPercent ?? window.capacityPercent - window.remainingPercent;
    return window.remainingPercent;
}

function memberWindow(members, accountId, windowId) {
    const member = members.find(account => account.id === accountId);
    return member?.windows.find(window => window.id === windowId) ?? null;
}

function segmentTick(members, segment, windowId, display) {
    const window = memberWindow(members, segment.accountId, windowId);
    return window === null ? null : Quota.tickPosition(window, display);
}

function segments(window, members, display) {
    return window.segments.map(segment => ({
        fraction: Quota.fillFraction(segment, display.valueMode),
        tone: Quota.meterTone(segment),
        tick: segmentTick(members, segment, window.id, display)
    }));
}

function segmentName(segment, members) {
    const member = members.find(account => account.id === segment.accountId);
    return segment.label ?? (member ? Providers.accountName(member) : segment.accountId);
}

function breakdown(lang, window, members, now, display) {
    return window.segments.map(segment => {
        const reading = Format.readingFor(lang, Quota.shownPercent(segment, display.valueMode), display.valueMode);
        const reset = Quota.trailingText(lang, segment, now, display.resetFormat, false);
        return `${segmentName(segment, members)}: ${reading} · ${reset}`;
    }).join("\n");
}
