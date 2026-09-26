import { isObject } from '../fields.js';

export function hasBreakdownSetting(rawSettings) {
    return (
        isObject(rawSettings) &&
        isObject(rawSettings.display) &&
        typeof rawSettings.display.show_breakdown === 'boolean'
    );
}
