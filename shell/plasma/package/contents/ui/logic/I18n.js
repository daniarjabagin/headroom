.pragma library

.import "Russian.js" as Russian

const LANGUAGES = ["system", "en", "ru"];

function resolve(language, localeName) {
    if (language === "en" || language === "ru")
        return language;
    return String(localeName ?? "").toLowerCase().startsWith("ru") ? "ru" : "en";
}

function fill(template, values) {
    return template.replace(/\{(\w+)\}/g, (match, name) => values && name in values ? String(values[name]) : match);
}

function entry(lang, msgid) {
    if (lang !== "ru")
        return null;
    return Russian.MESSAGES[msgid] ?? null;
}

function N(msgid) {
    return msgid;
}

function tr(lang, msgid, values) {
    const translated = entry(lang, msgid);
    return fill(typeof translated === "string" ? translated : msgid, values);
}

function pluralIndex(lang, count) {
    const whole = Math.abs(Math.trunc(count));
    if (lang !== "ru")
        return whole === 1 ? 0 : 1;
    const lastDigit = whole % 10;
    const lastTwo = whole % 100;
    if (lastDigit === 1 && lastTwo !== 11)
        return 0;
    if (lastDigit >= 2 && lastDigit <= 4 && (lastTwo < 12 || lastTwo > 14))
        return 1;
    return 2;
}

function trn(lang, singular, plural, count, values) {
    const translated = entry(lang, singular);
    const forms = Array.isArray(translated) ? translated : [singular, plural];
    const index = Math.min(pluralIndex(Array.isArray(translated) ? lang : "en", count), forms.length - 1);
    return fill(forms[index], Object.assign({
        count
    }, values));
}
