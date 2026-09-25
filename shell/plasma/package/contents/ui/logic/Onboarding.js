.pragma library

.import "I18n.js" as I18n
.import "State.js" as State

function pending(settings, capable) {
    return capable === true && settings !== null && settings !== undefined && settings.onboarding.completed === false;
}

function foundNames(state) {
    const names = [];
    for (const account of state === null ? [] : State.visibleAccounts(state))
        if (!names.includes(account.providerName))
            names.push(account.providerName);
    return names;
}

function joinedNames(lang, names) {
    if (names.length < 2)
        return names.join("");
    return I18n.tr(lang, "{list} and {last}", {
        list: names.slice(0, -1).join(", "),
        last: names[names.length - 1]
    });
}

function title(lang, state) {
    const names = foundNames(state);
    if (names.length === 0)
        return I18n.tr(lang, "Welcome to Headroom");
    return I18n.tr(lang, "Welcome to Headroom — we found {providers}", {
        providers: joinedNames(lang, names)
    });
}

function detail(lang) {
    return I18n.tr(lang, "Review what shows in the panel and popup, or keep the defaults.");
}
