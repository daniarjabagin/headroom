import { isObject } from '../fields.js';

export function breakdownSetting(rawSettings) {
    const value = isObject(rawSettings) && isObject(rawSettings.display) ? rawSettings.display.show_breakdown : null;
    if (typeof value !== 'boolean') return { supported: false, shown: true };
    return { supported: true, shown: value };
}

export function breakdownPatch(shown) {
    return { display: { show_breakdown: shown === true } };
}
