import { limitText, readingPercent, resetText, spareText } from './format.js';
import { _ } from './i18n.js';

const TONE_CLASSES = { good: 'ok', warning: 'warn', critical: 'crit', neutral: 'none' };

export function toneClass(tone) {
    return TONE_CLASSES[tone] ?? 'none';
}

export function hasData(window) {
    return window.remainingPercent !== null;
}

export function meterTone(window) {
    return hasData(window) ? toneClass(window.tone) : 'none';
}

export function fillFraction(window, valueMode) {
    if (!hasData(window)) return 0;
    return Math.min(1, Math.max(0, readingPercent(window, valueMode) / 100));
}

export function tickPosition(window, display) {
    const even = window.pace.evenPacePercent;
    if (even === null || !hasData(window)) return null;
    if (!display.showForecast && window.tone !== 'warning' && window.tone !== 'critical') return null;
    return display.valueMode === 'used' ? even / 100 : 1 - even / 100;
}

function combinedEstimate(window) {
    return window.capacityPercent !== undefined && window.pace.runsOutAt === null;
}

export function paceNote(window, now, showForecast) {
    const { severity, sparePercent, runsOutAt } = window.pace;
    if (severity === 'spent') return { flame: true, text: _('Limit reached') };
    if (severity === 'running_out')
        return {
            flame: true,
            text: showForecast || combinedEstimate(window) ? _('Over pace') : limitText(runsOutAt, now),
        };
    if (severity === 'close' && sparePercent !== null && !showForecast)
        return { flame: false, text: spareText(sparePercent) };
    return null;
}

export function trailingText(window, now, resetFormat) {
    if (!hasData(window)) return _('No data');
    return resetText(window.resetsAt, now, resetFormat, true);
}
