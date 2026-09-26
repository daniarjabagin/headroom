.pragma library

.import "Account.js" as Account
.import "I18n.js" as I18n

const NAME_SEPARATOR = ", ";
const TONE_RANK = {
    neutral: 0,
    good: 0,
    warning: 1,
    critical: 2
};
const CALM = {
    kind: "none",
    tone: "",
    count: 0
};

function isCollapsed(card) {
    return card.kind === "combined" ? card.group.collapsed === true : card.account.collapsed === true;
}

function partition(cards) {
    return {
        shown: cards.filter(card => !isCollapsed(card)),
        folded: cards.filter(isCollapsed)
    };
}

function cardName(card) {
    return card.kind === "combined" ? card.group.providerName : card.account.providerName;
}

function cardProvider(card) {
    return card.kind === "combined" ? card.group.provider : card.account.provider;
}

function distinct(values) {
    return values.filter((value, index, all) => all.indexOf(value) === index);
}

function foldedCount(lang, folded) {
    return I18n.trn(lang, "{count} more", "{count} more", folded.length);
}

function foldedNames(folded) {
    return distinct(folded.map(cardName)).join(NAME_SEPARATOR);
}

function foldedProviders(folded, limit) {
    return distinct(folded.map(cardProvider)).slice(0, limit);
}

function foldedTitle(lang, folded) {
    const names = foldedNames(folded);
    const more = foldedCount(lang, folded);
    return names === "" ? more : `${more} · ${names}`;
}

function accessibleTitle(lang, folded, attention) {
    const note = attentionText(lang, attention);
    const title = foldedTitle(lang, folded);
    return note === "" ? title : `${title}. ${note}`;
}

function cardAccounts(card) {
    return card.kind === "combined" ? card.members : [card.account];
}

function cardWindows(card) {
    const windows = card.kind === "combined" ? card.group.windows : card.account.windows;
    return windows.filter(window => !window.hidden);
}

function rank(tone) {
    return TONE_RANK[tone] ?? 0;
}

function worstTone(tones) {
    return tones.reduce((worst, tone) => rank(tone) > rank(worst) ? tone : worst, "neutral");
}

function accountNotice(account, offline) {
    const status = Account.settledStatus(account);
    if (status === "signed_out" || status === "no_subscription")
        return status;
    return status === "error" && !Account.failedOffline(account, offline) ? "error" : "";
}

function cardAttention(card, offline) {
    return {
        notices: cardAccounts(card).map(account => accountNotice(account, offline)).filter(notice => notice !== ""),
        tone: worstTone(cardWindows(card).map(window => window.tone))
    };
}

function attention(folded, offline) {
    const entries = folded.map(card => cardAttention(card, offline));
    const count = entries.filter(entry => entry.notices.length > 0 || rank(entry.tone) > 0).length;
    const notices = entries.reduce((all, entry) => all.concat(entry.notices), []);
    if (notices.length > 0)
        return {
            kind: "notice",
            tone: notices.includes("error") ? "critical" : "warning",
            count
        };
    const tone = worstTone(entries.map(entry => entry.tone));
    return rank(tone) > 0 ? {
        kind: "tone",
        tone,
        count
    } : CALM;
}

function attentionText(lang, attention) {
    return attention.kind === "none" ? "" : I18n.trn(lang, "{count} needs attention", "{count} need attention", attention.count);
}
