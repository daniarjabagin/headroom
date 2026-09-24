import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import { money } from '../src/numbers.js';
import { iconCandidates, seriesKey } from '../src/providers.js';
import { parseState } from '../src/state.js';
import { check } from './check.js';

const NEW_PROVIDERS = ['kilo', 'warp', 'poe', 'deepseek', 'moonshot'];
const LOGO_PROVIDERS = ['warp', 'poe', 'deepseek', 'moonshot'];

function iconExists(file) {
    const [testFile] = GLib.filename_from_uri(import.meta.url);
    const path = GLib.build_filenamev([GLib.path_get_dirname(testFile), '..', 'icons', file]);
    return Gio.File.new_for_path(path).query_exists(null);
}

function firstIcon(id) {
    return iconCandidates(id).find(candidate => iconExists(candidate.file))?.file ?? null;
}

function testMoneyFormat() {
    check('money cny', money('CNY', 12_500_000), '¥12.50');
    check('money usd', money('USD', 1_234_567_890), '$1,234.57');
    check('money eur', money('EUR', 990_000), '€0.99');
    check('money other code', money('GBP', 12_500_000), '12.50 GBP');
    check('money negative', money('CNY', -3_000_000), '-¥3.00');
    check('money negative other', money('GBP', -1_234_567_890), '-1,234.57 GBP');
    check('money half away from zero', [money('CNY', 5_000), money('CNY', -5_000)], ['¥0.01', '-¥0.01']);
    check('money below half cent', [money('CNY', 4_999), money('CNY', -4_999)], ['¥0.00', '¥0.00']);
    check('money zero', money('CNY', 0), '¥0.00');
    check('money grouping', money('CNY', 1_234_567_000_000), '¥1,234,567.00');
}

const BALANCES = JSON.stringify({
    version: 1,
    accounts: [
        {
            id: 'deepseek:1',
            provider: 'deepseek',
            balances: [
                { id: 'balance_cny', label: 'Balance', kind: 'money', currency: 'CNY', micros: 12_500_000 },
                { id: 'balance_usd', label: 'Balance', kind: 'money', currency: 'USD', micros: -3_000_000 },
                { id: 'bad', label: 'Bad', kind: 'money', currency: 'yuan', micros: 1 },
                { id: 'future', label: 'Future', kind: 'gems', micros: 1 },
            ],
        },
    ],
});

function testMoneyParse() {
    const [cny, usd, bad, future] = parseState(BALANCES).accounts[0].balances;
    check('money balance', cny, {
        id: 'balance_cny',
        label: 'Balance',
        kind: 'money',
        usdMicros: null,
        currency: 'CNY',
        micros: 12_500_000,
        value: null,
        unit: null,
    });
    check('money keeps sign and currency', [usd.currency, usd.micros], ['USD', -3_000_000]);
    check('money bad currency', [bad.kind, bad.currency], ['money', null]);
    check('unknown kind', [future.kind, future.micros], [null, null]);
}

function testNewProviders() {
    check('new provider series', NEW_PROVIDERS.map(seriesKey), NEW_PROVIDERS);
    check(
        'new provider logos',
        LOGO_PROVIDERS.map(firstIcon),
        LOGO_PROVIDERS.map(id => `${id}-symbolic.svg`)
    );
    check('kilo uses the generic icon', firstIcon('kilo'), null);
}

export function testMoney() {
    testMoneyFormat();
    testMoneyParse();
    testNewProviders();
}
