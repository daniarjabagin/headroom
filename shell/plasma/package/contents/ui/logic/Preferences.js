.pragma library

.import "I18n.js" as I18n
.import "Options.js" as Options
.import "Providers.js" as Providers
.import "SettingsValues.js" as Values
.import "State.js" as State

const SPEND_PERIODS = [["today", I18n.N("Today")], ["yesterday", I18n.N("Yesterday")], ["7d", I18n.N("7 Days")], ["30d", I18n.N("30 Days")]];

function sections(capable) {
    return capable ? Options.SECTIONS.filter(section => section.key !== "showSpend") : Options.SECTIONS;
}

function spendPeriodOptions(lang) {
    return Options.choices(lang, SPEND_PERIODS);
}

function reducedMotionPatch(enabled) {
    return {
        reduced_motion: enabled === true
    };
}

function limitChoices(lang, state, limits) {
    const options = Options.windowOptions(lang, state);
    const missing = limits.map(Options.panelLimitKey).filter(key => !options.some(option => option.value === key));
    return options.concat(missing.map(value => ({
                    value,
                    label: I18n.tr(lang, "Pinned limit (not available now)")
                })));
}

function providerOf(value) {
    return String(value).split(":")[0];
}

function isChosen(limits, value) {
    return limits.some(limit => Options.panelLimitKey(limit) === value);
}

function canChoose(limits, value) {
    return isChosen(limits, value) || limits.length < Values.MAX_PANEL_LIMITS;
}

function toggledLimits(limits, value, chosen) {
    const others = limits.filter(limit => Options.panelLimitKey(limit) !== value);
    if (!chosen)
        return others;
    return others.length < Values.MAX_PANEL_LIMITS ? others.concat([Options.panelLimitFor(value)]) : others;
}

function limitsSummary(lang, limits) {
    if (limits.length === 0)
        return I18n.tr(lang, "None chosen · the two most critical are shown");
    return I18n.tr(lang, "{count} of {max} chosen · shown in this order", {
        count: limits.length,
        max: Values.MAX_PANEL_LIMITS
    });
}

function accountSubtitle(account) {
    const title = Providers.accountName(account);
    return [account.providerName, account.plan, account.email].filter(part => part && part !== title).join(" · ");
}

function starRows(state) {
    const accounts = state === null ? [] : State.visibleAccounts(state);
    return accounts.map(account => ({
                id: account.id,
                provider: account.provider,
                title: Providers.accountName(account),
                subtitle: accountSubtitle(account)
            }));
}

function starLabel(lang, starred) {
    return starred ? I18n.tr(lang, "Always open") : I18n.tr(lang, "On demand");
}
