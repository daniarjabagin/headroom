.pragma library

.import "FormatSpend.js" as FormatSpend
.import "I18n.js" as I18n

const KINDS = {
    status: {
        icon: "network-connect",
        label: I18n.N("Status page")
    },
    dashboard: {
        icon: "link",
        label: I18n.N("Open dashboard")
    },
    usage: {
        icon: "view-statistics",
        label: I18n.N("Usage page")
    }
};

const HEADER_ORDER = ["status", "usage", "dashboard"];
const MENU_ORDER = ["status", "dashboard", "usage"];
const SCHEME = /^https:\/\//;
const WWW = /^www\./;
const MENU_HOST_CHARS = 40;

function host(url) {
    return url.replace(SCHEME, "").split(/[/?#]/)[0].replace(WWW, "");
}

function available(links, kind) {
    const url = links?.[kind] ?? null;
    if (url === null)
        return false;
    return kind !== "usage" || url !== links.dashboard;
}

function entry(lang, links, kind) {
    const label = I18n.tr(lang, KINDS[kind].label);
    return {
        kind,
        url: links[kind],
        icon: KINDS[kind].icon,
        label,
        host: host(links[kind]),
        menuHost: FormatSpend.middleEllipsis(host(links[kind]), MENU_HOST_CHARS),
        tip: `${label} · ${host(links[kind])}`
    };
}

function entries(lang, links, order) {
    return order.filter(kind => available(links, kind)).map(kind => entry(lang, links, kind));
}

function headerEntries(lang, links) {
    return entries(lang, links, HEADER_ORDER);
}

function menuEntries(lang, links) {
    return entries(lang, links, MENU_ORDER);
}
