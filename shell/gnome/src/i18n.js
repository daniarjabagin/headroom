import { RU } from './locale/ru.js';

const CATALOGS = { ru: RU };
const CONTEXT_SEPARATOR = '\u0004';
const current = { language: 'en' };

export function resolveLanguage(setting, languageNames) {
    if (setting === 'en' || setting === 'ru') return setting;
    return languageNames[0]?.startsWith('ru') ? 'ru' : 'en';
}

export function setLanguage(language) {
    current.language = language in CATALOGS ? language : 'en';
}

export function currentLanguage() {
    return current.language;
}

export function pluralIndex(language, count) {
    const whole = Math.abs(Math.trunc(count));
    if (language !== 'ru') return whole === 1 ? 0 : 1;
    const lastDigit = whole % 10;
    const lastTwo = whole % 100;
    if (lastDigit === 1 && lastTwo !== 11) return 0;
    if (lastDigit >= 2 && lastDigit <= 4 && (lastTwo < 12 || lastTwo > 14)) return 1;
    return 2;
}

function lookup(key) {
    return CATALOGS[current.language]?.[key];
}

export function _(msgid) {
    const translated = lookup(msgid);
    return typeof translated === 'string' ? translated : msgid;
}

export function C_(context, msgid) {
    const translated = lookup(contextKey(context, msgid));
    return typeof translated === 'string' ? translated : msgid;
}

export function n_(singular, plural, count) {
    const forms = lookup(singular);
    const index = pluralIndex(current.language, count);
    if (Array.isArray(forms)) return forms[Math.min(index, forms.length - 1)];
    return index === 0 ? singular : plural;
}

export function contextKey(context, msgid) {
    return `${context}${CONTEXT_SEPARATOR}${msgid}`;
}

export function fill(template, values) {
    return template.replace(/\{(\w+)\}/g, (match, name) => (name in values ? String(values[name]) : match));
}
