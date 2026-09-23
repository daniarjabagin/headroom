import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import { dayTitle } from '../src/dates.js';
import * as format from '../src/format.js';
import { contextKey, pluralIndex, resolveLanguage, setLanguage } from '../src/i18n.js';
import { RU } from '../src/locale/ru.js';
import * as numbers from '../src/numbers.js';
import { check } from './check.js';
import { literalErrorFiles, scanMessages } from './messageScan.js';

const NBSP = '\u00a0';

function extensionDir() {
    const [testFile] = GLib.filename_from_uri(import.meta.url);
    return Gio.File.new_for_path(GLib.path_get_dirname(testFile)).get_parent();
}

function testPlurals() {
    const ru = [1, 2, 5, 11, 12, 14, 21, 22, 25, 101, 111, 0].map(count => pluralIndex('ru', count));
    check('ru plural forms', ru, [0, 1, 2, 2, 2, 2, 0, 1, 2, 0, 2, 2]);
    check(
        'en plural forms',
        [1, 0, 2].map(count => pluralIndex('en', count)),
        [0, 1, 1]
    );
    check('resolve ru', resolveLanguage('system', ['ru_RU.UTF-8', 'ru_RU', 'ru', 'C']), 'ru');
    check('resolve en', resolveLanguage('system', ['en_US.UTF-8', 'en', 'C']), 'en');
    check('resolve C', resolveLanguage('system', ['C']), 'en');
    check('resolve forced', [resolveLanguage('en', ['ru']), resolveLanguage('ru', ['C'])], ['en', 'ru']);
}

function testRussianReadings(now) {
    check('ru left', format.percentLeft(61.6), 'Осталось 62%');
    check('ru used', format.percentUsed(38.4), 'Использовано 38%');
    check('ru reset', format.resetText(new Date('2026-09-23T12:41:00Z'), now), 'Сброс через 2 ч 41 мин');
    check('ru reset pending', format.resetText(new Date('2026-09-23T09:00:00Z'), now, 'exact'), 'Ожидается сброс');
    check('ru reset days', format.resetText(new Date('2026-09-27T15:30:00Z'), now), 'Сброс через 4 д 5 ч');
    check(
        'ru live',
        format.resetText(new Date('2026-09-23T10:12:07Z'), now, 'countdown', true),
        'Сброс через 12 мин 07 с'
    );
    check('ru not started', format.resetText(null, now), 'Не начато');
    check('ru limit', format.limitText(new Date('2026-09-24T19:00:00Z'), now), 'Лимит через 1 д 9 ч');
    check('ru spare', format.spareText(4.2), '~4% запаса');
    check('ru next update', format.nextUpdateText(new Date('2026-09-23T10:03:10Z'), now), 'Обновление через 3 мин');
    check('ru session', format.windowLabel('session', 'Session'), 'Сессия');
}

function testRussianForecast(now) {
    const pace = { evenPacePercent: 50, projectedPercent: null, sparePercent: null, runsOutAt: null };
    const runningOut = {
        remainingPercent: 29,
        resetsAt: new Date('2026-09-23T12:28:00Z'),
        pace: { ...pace, severity: 'running_out', runsOutAt: new Date('2026-09-23T10:45:00Z') },
    };
    const display = { valueMode: 'left', resetFormat: 'countdown' };
    check(
        'ru forecast run out',
        format.forecastText(runningOut, now, display),
        'При текущем темпе: закончится через 45 мин · сброс через 2 ч 28 мин'
    );
    const healthy = { ...runningOut, pace: { ...pace, severity: 'healthy', projectedPercent: 60, sparePercent: 40 } };
    check('ru forecast spare', format.forecastText(healthy, now, display), 'При текущем темпе к сбросу останется ~40%');
}

function testRussianDates() {
    const now = new Date(2026, 8, 23, 10, 0);
    const at = (month, day, hour, minute) => new Date(2026, month, day, hour, minute);
    check('ru today', format.resetText(at(8, 23, 17, 5), now, 'exact'), 'Сброс сегодня в 17:05');
    check('ru tomorrow', format.resetText(at(8, 24, 9, 30), now, 'exact'), 'Сброс завтра в 09:30');
    check('ru weekday', format.resetText(at(8, 26, 18, 0), now, 'exact'), 'Сброс в субботу в 18:00');
    check('ru date', format.resetText(at(9, 3, 14, 0), now, 'exact'), 'Сброс 3 окт. в 14:00');
    check('ru day title', dayTitle('2026-09-23'), 'ср, 23 сент.');
}

function testRussianNumbers() {
    check('ru compact', numbers.compactTokens(1_203_448), `1,2${NBSP}млн`);
    check(
        'ru spend',
        numbers.spendLine({ costMicros: 18_420_000, totalTokens: 1_203_448 }),
        `$18.42 · 1,2${NBSP}млн токенов`
    );
    check('ru one token', numbers.exactTokensText(1), '1 токен');
    check('ru few tokens', numbers.exactTokensText(3), '3 токена');
    check('ru exact tokens', numbers.exactTokensText(35_812_904), `35${NBSP}812${NBSP}904 токена`);
    check('ru money', numbers.exactUsd(1_234_567_890), '$1,234.57');
}

function testCatalogCoverage() {
    const messages = scanMessages(extensionDir());
    const missing = messages.plain.filter(msgid => typeof RU[msgid] !== 'string');
    check('ru covers every msgid', missing, []);
    const missingContext = messages.context.filter(
        ([context, msgid]) => typeof RU[contextKey(context, msgid)] !== 'string'
    );
    check('ru covers every context msgid', missingContext, []);
    const badPlurals = messages.plural.filter(msgid => !Array.isArray(RU[msgid]) || RU[msgid].length !== 3);
    check('ru plural entries have three forms', badPlurals, []);
    check('only literal msgids', messages.dynamic, []);
    check('prefs errors are translated', literalErrorFiles(extensionDir()), []);
    const used = new Set([
        ...messages.plain,
        ...messages.plural,
        ...messages.context.map(([context, msgid]) => contextKey(context, msgid)),
    ]);
    check(
        'ru has no unused entries',
        Object.keys(RU).filter(key => !used.has(key)),
        []
    );
}

export function testLocale() {
    testPlurals();
    testCatalogCoverage();
    const now = new Date('2026-09-23T10:00:00Z');
    setLanguage('ru');
    try {
        testRussianReadings(now);
        testRussianForecast(now);
        testRussianDates();
        testRussianNumbers();
    } finally {
        setLanguage('en');
    }
    check('en day title', dayTitle('2026-09-23'), 'Wed, Sep 23');
}
