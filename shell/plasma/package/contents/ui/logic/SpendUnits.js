.pragma library

.import "FormatSpend.js" as FormatSpend
.import "I18n.js" as I18n

const UNITS = [
    {
        value: "cost",
        title: I18n.N("Total Spend"),
        subtitle: I18n.N("Dollars, from local logs and public prices")
    },
    {
        value: "tokens",
        title: I18n.N("Total Tokens"),
        subtitle: I18n.N("Input, output and cache tokens")
    },
    {
        value: "cost_per_mtok",
        title: I18n.N("Cost per MTok"),
        subtitle: I18n.N("Spend divided by million tokens")
    }
];

function unitFor(value) {
    return UNITS.find(unit => unit.value === value) ?? UNITS[0];
}

function unitOptions(lang) {
    return UNITS.map(unit => ({
                value: unit.value,
                title: I18n.tr(lang, unit.title),
                subtitle: I18n.tr(lang, unit.subtitle)
            }));
}

function unitTitle(lang, value) {
    return I18n.tr(lang, unitFor(value).title);
}

function ringCaption(lang, value) {
    if (value === "tokens")
        return I18n.tr(lang, "tokens");
    if (value === "cost_per_mtok")
        return I18n.tr(lang, "blended");
    return "";
}

function ringValue(period, value) {
    return FormatSpend.unitValue(period, unitFor(value).value);
}

function legendValue(entry, value) {
    const unit = unitFor(value).value;
    return unit === "cost" ? FormatSpend.usd(entry.costMicros) : FormatSpend.unitValue(entry, unit);
}

function byTokens(value) {
    return unitFor(value).value !== "cost";
}
