.pragma library

.import "I18n.js" as I18n

const NAME_SEPARATOR = ", ";

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
