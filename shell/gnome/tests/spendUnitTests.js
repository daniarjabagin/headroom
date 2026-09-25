import { DiagnosticsError, parseDiagnostics } from '../src/diagnostics.js';
import { setLanguage } from '../src/i18n.js';
import { parseSpendResult, spendQuery, SpendQueryError } from '../src/spendQuery.js';
import * as units from '../src/spendUnits.js';
import { check, throws } from './check.js';

const ENTRY = { costMicros: 12_400_000, totalTokens: 5_710_000, costPerMtokMicros: 2_171_629 };

function testUnits() {
    check('cost per mtok', units.costPerMtokText(2_171_629), '$2.17/MTok');
    check('cost per mtok small', units.costPerMtokText(4_300), '$0.00/MTok');
    check('cost per mtok large', units.costPerMtokText(1_234_000_000), '$1.23K/MTok');
    check('cost per mtok missing', units.costPerMtokText(null), '—');
    check(
        'unit texts',
        ['cost', 'tokens', 'cost_per_mtok'].map(unit => units.spendValueText(ENTRY, unit)),
        ['$12.40', '5.7M', '$2.17/MTok']
    );
    check(
        'unit values',
        ['cost', 'tokens', 'cost_per_mtok'].map(unit => units.spendValue(ENTRY, unit)),
        [12_400_000, 5_710_000, 2_171_629]
    );
    check('unpriced value', units.spendValue({ ...ENTRY, costPerMtokMicros: null }, 'cost_per_mtok'), null);
    setLanguage('ru');
    try {
        check('ru cost per mtok', units.costPerMtokText(2_171_629), '$2.17 за 1 млн токенов');
        check('ru no project', units.projectLabel(null, 20), 'Без проекта');
    } finally {
        setLanguage('en');
    }
}

function testProjects() {
    check('short path kept', units.ellipsizedMiddle('~/code/app', 20), '~/code/app');
    check('middle ellipsis', units.ellipsizedMiddle('~/work/clients/acme/backend', 16), '~/work/…/backend');
    check('exact fit', units.ellipsizedMiddle('abcdef', 6), 'abcdef');
    check('tiny budget', units.ellipsizedMiddle('abcdef', 1), '…');
    check('code points', units.ellipsizedMiddle('~/проекты/приложение', 10), '~/пр…жение');
    check('no project', units.projectLabel(null, 20), 'No project');
    check('project label', units.projectLabel('~/code/headroom', 40), '~/code/headroom');
    check('other projects', units.otherProjectsLabel({ count: 3 }), 'Other (3)');
}

function testPeriods() {
    const spend = { today: { projects: [] }, yesterday: {}, last7Days: null, last30Days: { projects: null } };
    check('periods without 7d', units.spendPeriods(spend), ['today', 'yesterday', '30d']);
    check('periods with 7d', units.spendPeriods({ ...spend, last7Days: {} }), ['today', 'yesterday', '7d', '30d']);
    check('period fallback', units.spendPeriod(spend, '7d'), spend.last30Days);
    check('period today', units.spendPeriod(spend, 'today'), spend.today);
    check('no spend', [units.spendPeriods(null), units.spendPeriod(null, '30d')], [[], null]);
    check('has projects', [units.hasProjects(spend.today), units.hasProjects(spend.last30Days)], [true, false]);
}

function testQueries() {
    check('query period', spendQuery({ period: '7d', by: 'model' }), { period: '7d', by: 'model' });
    check('query range', spendQuery({ since: '2026-09-17', until: '2026-09-23', by: 'day', provider: 'claude' }), {
        since: '2026-09-17',
        until: '2026-09-23',
        by: 'day',
        provider: 'claude',
    });
    check('query open range', spendQuery({ since: '2026-09-17', by: 'project' }), {
        since: '2026-09-17',
        by: 'project',
    });
    throws('query both', () => spendQuery({ period: '7d', since: '2026-09-17', by: 'model' }), SpendQueryError);
    throws('query neither', () => spendQuery({ by: 'model' }), SpendQueryError);
    throws('query bad by', () => spendQuery({ period: '7d', by: 'session' }), SpendQueryError);
    throws('query bad period', () => spendQuery({ period: '90d', by: 'model' }), SpendQueryError);
    throws('query until alone', () => spendQuery({ period: '7d', until: '2026-09-23', by: 'model' }), SpendQueryError);
    throws(
        'query reversed',
        () => spendQuery({ since: '2026-09-23', until: '2026-09-17', by: 'day' }),
        SpendQueryError
    );
    throws('query empty provider', () => spendQuery({ period: 'today', by: 'model', provider: '' }), SpendQueryError);
}

function testResults() {
    const row = {
        key: 'claude-opus-4-5',
        provider: 'claude',
        tokens: { input: 1, cache_read: 2, cache_write: 3, output: 4, reasoning: 1, total: 10 },
        cost_usd_micros: 151_200_000,
        partial: false,
        unpriced_tokens: 0,
        cost_per_mtok_usd_micros: 786_271,
        share_permille: 767,
    };
    const result = parseSpendResult(
        JSON.stringify({
            since: '2026-09-17',
            until: '2026-09-23',
            by: 'model',
            rows: [row, 5],
            total: { ...row, key: null },
        })
    );
    check('result rows', result.rows.length, 1);
    check(
        'result row',
        [
            result.rows[0].key,
            result.rows[0].totalTokens,
            result.rows[0].costPerMtokMicros,
            result.rows[0].sharePermille,
        ],
        ['claude-opus-4-5', 10, 786_271, 767]
    );
    check('result total key', [result.total.key, result.total.provider], [null, 'claude']);
    throws('result garbage', () => parseSpendResult('nope'), SpendQueryError);
    throws('result wrong by', () => parseSpendResult('{"by":"session"}'), SpendQueryError);
}

function testDiagnostics() {
    const report = parseDiagnostics(
        JSON.stringify({
            app_version: '0.6.0',
            os: 'Arch Linux',
            desktop: null,
            uptime_secs: 7530,
            transports: ['dbus', 3],
            log_level: 'info',
            log_level_source: 'settings',
            log_file: '~/.local/state/headroom/headroom.log',
            providers: [{ provider: 'claude', accounts: 1, usage_homes: 1 }],
            text: 'Headroom 0.6.0\n',
        })
    );
    check('diagnostics text', report.text, 'Headroom 0.6.0\n');
    check('diagnostics log file', report.logFile, '~/.local/state/headroom/headroom.log');
    check('diagnostics fields', [report.desktop, report.transports, report.uptimeSecs], [null, ['dbus'], 7530]);
    check('diagnostics providers', report.providers, [{ provider: 'claude', accounts: 1, usageHomes: 1 }]);
    throws('diagnostics without text', () => parseDiagnostics('{"app_version":"0.6.0"}'), DiagnosticsError);
    throws('diagnostics garbage', () => parseDiagnostics('{'), DiagnosticsError);
}

export function testSpendUnits() {
    testUnits();
    testProjects();
    testPeriods();
    testQueries();
    testResults();
    testDiagnostics();
}
