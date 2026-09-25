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

function foldedTitle(lang, folded) {
    const names = folded.map(cardName).filter((name, index, all) => all.indexOf(name) === index);
    const more = I18n.trn(lang, "{count} more", "{count} more", folded.length);
    return names.length === 0 ? more : `${more} · ${names.join(NAME_SEPARATOR)}`;
}
