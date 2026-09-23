import { C_, _, fill } from './i18n.js';

const DAY = 24 * 60 * 60 * 1000;
const WEEK_DAYS = 7;
const ISO_DATE = /^(\d{4})-(\d{2})-(\d{2})$/;

const WEEKDAYS_ON = () => [
    C_('on weekday', 'Sun'),
    C_('on weekday', 'Mon'),
    C_('on weekday', 'Tue'),
    C_('on weekday', 'Wed'),
    C_('on weekday', 'Thu'),
    C_('on weekday', 'Fri'),
    C_('on weekday', 'Sat'),
];

const WEEKDAYS_SHORT = () => [
    C_('short weekday', 'Sun'),
    C_('short weekday', 'Mon'),
    C_('short weekday', 'Tue'),
    C_('short weekday', 'Wed'),
    C_('short weekday', 'Thu'),
    C_('short weekday', 'Fri'),
    C_('short weekday', 'Sat'),
];

const MONTHS = () => [
    _('Jan'),
    _('Feb'),
    _('Mar'),
    _('Apr'),
    _('May'),
    _('Jun'),
    _('Jul'),
    _('Aug'),
    _('Sep'),
    _('Oct'),
    _('Nov'),
    _('Dec'),
];

export function clockTime(date) {
    const hours = String(date.getHours()).padStart(2, '0');
    const minutes = String(date.getMinutes()).padStart(2, '0');
    return `${hours}:${minutes}`;
}

function startOfDay(date) {
    return new Date(date.getFullYear(), date.getMonth(), date.getDate());
}

function calendarDaysBetween(from, to) {
    return Math.round((startOfDay(to) - startOfDay(from)) / DAY);
}

function monthDay(date) {
    return fill(_('{month} {date}'), { month: MONTHS()[date.getMonth()], date: date.getDate() });
}

export function exactMoment(date, now) {
    const time = clockTime(date);
    const days = calendarDaysBetween(now, date);
    if (days <= 0) return fill(_('today at {time}'), { time });
    if (days === 1) return fill(_('tomorrow at {time}'), { time });
    if (days < WEEK_DAYS) return fill(_('{weekday} at {time}'), { weekday: WEEKDAYS_ON()[date.getDay()], time });
    return fill(_('{day} at {time}'), { day: monthDay(date), time });
}

function parseIsoDate(text) {
    const match = ISO_DATE.exec(text);
    if (!match) return null;
    return new Date(Number(match[1]), Number(match[2]) - 1, Number(match[3]));
}

export function dayTitle(text) {
    const date = parseIsoDate(text);
    if (date === null) return text;
    return fill(_('{weekday}, {day}'), { weekday: WEEKDAYS_SHORT()[date.getDay()], day: monthDay(date) });
}
