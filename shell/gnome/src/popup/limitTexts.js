import { exactMoment } from '../dates.js';
import { duration, forecastText, resetText } from '../format.js';
import { _ } from '../i18n.js';
import { hasData, paceNote } from '../quota.js';

const SECOND = 1000;

export function trailingLabel(window, now, resetFormat, hour12) {
    if (!hasData(window)) return _('No data');
    return resetText(window.resetsAt, now, resetFormat, true, hour12);
}

function capitalized(text) {
    return text.charAt(0).toUpperCase() + text.slice(1);
}

export function compactTrailing(window, now, resetFormat, hour12) {
    if (!hasData(window)) return _('No data');
    if (window.resetsAt === null) return _('Not started');
    if (window.resetsAt <= now) return capitalized(_('reset pending'));
    if (resetFormat === 'exact') return exactMoment(window.resetsAt, now, hour12);
    const left = window.resetsAt - now;
    return left < SECOND ? capitalized(_('resets soon')) : duration(left, true);
}

export function valueHint(valueMode) {
    return valueMode === 'used' ? _('Click to show what is left') : _('Click to show used');
}

export function resetHint(resetFormat) {
    return resetFormat === 'exact' ? _('Click to show the countdown') : _('Click to show the reset time');
}

export function paceTip(window, now, display, hour12) {
    const note = paceNote(window, now, display.showForecast)?.text ?? null;
    const forecast = forecastText(window, now, display, hour12);
    const lines = [note, forecast].filter(Boolean);
    return lines.length > 0 ? lines.join('\n') : null;
}
