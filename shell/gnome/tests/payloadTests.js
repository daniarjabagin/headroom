import { parseProviders } from '../src/prefs/registry.js';
import { providerIncident } from '../src/providerStatus.js';
import { parseState } from '../src/state.js';
import { check } from './check.js';

const HEADLINE = {
    account_id: 'claude:1',
    window: 'weekly',
    provider: 'claude',
    provider_name: 'Claude',
    account_label: 'ada',
    window_label: 'Weekly',
    used_percent: 28,
    remaining_percent: 72,
    tone: 'warning',
};

function state(extra) {
    return parseState(JSON.stringify({ version: 1, ...extra }));
}

function testPanelItems() {
    const item = { ...HEADLINE, value_percent: 72, even_pace_percent: 41.5, logo: 'claude', combined: false };
    const parsed = state({
        panel_items: [item, { ...item, logo: '../evil' }, { tone: 'good' }],
        panel_tone: 'critical',
    });
    check('panel item count', parsed.panelItems.length, 2);
    check(
        'panel item extras',
        [parsed.panelItems[0].valuePercent, parsed.panelItems[0].evenPacePercent, parsed.panelItems[0].logo],
        [72, 41.5, 'claude']
    );
    check('panel item unsafe logo', parsed.panelItems[1].logo, null);
    check('panel tone', parsed.panelTone, 'critical');
    check('reports panel items', parsed.reportsPanelItems, true);
    const empty = state({ panel_items: [], panel_tone: null, headline: HEADLINE });
    check('icon mode keeps empty items', empty.panelItems, []);
    check('null panel tone', empty.panelTone, null);
    const many = state({ panel_items: [item, item, item, item] });
    check('at most three items', many.panelItems.length, 3);
}

function testPanelFallback() {
    const legacy = state({ headline: HEADLINE, display: { value_mode: 'used' } });
    check('legacy reports nothing', legacy.reportsPanelItems, false);
    check(
        'legacy item from headline',
        legacy.panelItems.map(entry => [entry.accountId, entry.valuePercent, entry.logo, entry.evenPacePercent]),
        [['claude:1', 28, 'claude', null]]
    );
    check('legacy tone from headline', legacy.panelTone, 'warning');
    const nothing = state({});
    check('legacy without headline', [nothing.panelItems, nothing.panelTone], [[], null]);
    check('bad panel tone', state({ panel_items: [], panel_tone: 'purple' }).panelTone, null);
    check('panel items not a list', state({ panel_items: 'x', headline: HEADLINE }).panelItems.length, 1);
}

function testAccountAdditions() {
    const refresh = { mode: 'live', interval_secs: 60, next_at: '2026-09-23T10:01:00Z', reason: 'activity' };
    const parsed = state({
        accounts: [
            { id: 'claude:1', provider: 'claude', collapsed: true, refresh },
            { id: 'codex:2', provider: 'codex', refresh: { mode: 'warp', interval_secs: 5, next_at: 'soon' } },
            { id: 'grok:3', provider: 'grok', collapsed: 'yes' },
        ],
        combined: [{ provider: 'codex', account_ids: ['codex:2'], collapsed: true }, { account_ids: ['x'] }],
    });
    const [live, odd, bare] = parsed.accounts;
    check(
        'refresh live',
        [live.refresh.mode, live.refresh.intervalSecs, live.refresh.reason],
        ['live', 60, 'activity']
    );
    check('refresh next', live.refresh.nextAt.toISOString(), '2026-09-23T10:01:00.000Z');
    check('collapsed', live.collapsed, true);
    check('malformed refresh', odd.refresh, { mode: 'idle', intervalSecs: null, nextAt: null, reason: 'schedule' });
    check('missing refresh', bare.refresh, null);
    check('non-bool collapsed', bare.collapsed, false);
    check(
        'combined collapsed',
        parsed.combined.map(group => group.collapsed),
        [true, false]
    );
}

function testProviderStatus() {
    const parsed = state({
        provider_status: [
            {
                provider: 'claude',
                indicator: 'minor',
                tone: 'warning',
                title: 'Elevated errors',
                stage: 'identified',
                started_at: '2026-09-23T09:12:00Z',
                url: 'https://stspg.io/abc123',
            },
            { provider: 'codex', indicator: 'none', tone: 'neutral', url: 'http://status.openai.com' },
            { provider: 'copilot', indicator: 'major' },
            { provider: 'grok', indicator: 'exploded' },
            { indicator: 'minor' },
        ],
    });
    check(
        'status entries',
        parsed.providerStatus.map(entry => [entry.provider, entry.indicator, entry.tone]),
        [
            ['claude', 'minor', 'warning'],
            ['codex', 'none', 'neutral'],
            ['copilot', 'major', 'critical'],
        ]
    );
    check('status started', parsed.providerStatus[0].startedAt.toISOString(), '2026-09-23T09:12:00.000Z');
    check('status https only', parsed.providerStatus[1].url, null);
    check('status defaults', [parsed.providerStatus[2].title, parsed.providerStatus[2].stage], [null, null]);
    check('incident lookup', providerIncident(parsed.providerStatus, 'claude').title, 'Elevated errors');
    check('clear is no incident', providerIncident(parsed.providerStatus, 'codex'), null);
    check('missing status', state({}).providerStatus, []);
    check('malformed status', state({ provider_status: {} }).providerStatus, []);
}

const PERIOD = {
    cost_usd_micros: 1000,
    total_tokens: 10,
    cost_per_mtok_usd_micros: 217163,
    by_provider: [{ provider: 'claude', cost_per_mtok_usd_micros: null, models: [{ model: 'm' }] }],
    projects: [
        {
            project: '~/code/headroom',
            cost_usd_micros: 900,
            total_tokens: 9,
            share_permille: 733,
            by_provider: [{ provider: 'claude', provider_name: 'Claude', cost_usd_micros: 900, total_tokens: 9 }],
        },
        { project: null, cost_usd_micros: 100, total_tokens: 1, partial: true },
    ],
    projects_other: { count: 4, cost_usd_micros: 3, total_tokens: 2, share_permille: 266 },
};

function testSpendAdditions() {
    const usage = [{ provider: 'claude' }];
    const parsed = state({ usage, spend: { today: PERIOD, last_7_days: PERIOD } }).spend;
    check('7 days present', parsed.last7Days.costPerMtokMicros, 217163);
    check('provider cost per mtok missing', parsed.today.providers[0].costPerMtokMicros, null);
    check('model cost per mtok missing', parsed.today.providers[0].models[0].costPerMtokMicros, null);
    check(
        'projects',
        parsed.today.projects.map(entry => [entry.project, entry.sharePermille, entry.partial]),
        [
            ['~/code/headroom', 733, false],
            [null, 0, true],
        ]
    );
    check('project providers', parsed.today.projects[0].providers[0].providerName, 'Claude');
    check('projects other', parsed.today.projectsOther, {
        count: 4,
        costMicros: 3,
        totalTokens: 2,
        partial: false,
        sharePermille: 266,
    });
    const legacy = state({ usage, spend: { today: { cost_usd_micros: 5 } } }).spend;
    check('legacy 7 days', legacy.last7Days, null);
    check('legacy projects', [legacy.today.projects, legacy.today.projectsOther], [null, null]);
    check('legacy cost per mtok', legacy.today.costPerMtokMicros, null);
    const odd = state({ usage, spend: { today: { cost_per_mtok_usd_micros: 1.5, projects: 'x' } } }).spend;
    check('fractional cost per mtok rejected', odd.today.costPerMtokMicros, null);
    check('malformed projects', odd.today.projects, null);
}

function testLinks() {
    const providers = parseProviders(
        JSON.stringify({
            version: 1,
            providers: [
                {
                    id: 'copilot',
                    add_account: [{ kind: 'cli_login', program: 'gh' }],
                    links: {
                        status: 'https://www.githubstatus.com',
                        dashboard: 'javascript:alert(1)',
                        usage: 'https://github.com/settings/billing/summary',
                    },
                },
                { id: 'codex', add_account: [{ kind: 'cli_login', program: 'codex' }] },
            ],
        })
    );
    check('links', providers[0].links, {
        status: 'https://www.githubstatus.com',
        dashboard: null,
        usage: 'https://github.com/settings/billing/summary',
    });
    check('missing links', providers[1].links, { status: null, dashboard: null, usage: null });
}

export function testSampleAdditions(sample) {
    check('sample reports panel items', sample.reportsPanelItems, true);
    check('sample panel item', [sample.panelItems[0].valuePercent, sample.panelItems[0].logo], [62, 'codex']);
    check('sample panel tone', sample.panelTone, 'critical');
    check('sample incident', providerIncident(sample.providerStatus, 'claude').stage, 'identified');
    check(
        'sample refresh modes',
        sample.accounts.map(account => account.refresh.mode),
        ['idle', 'idle', 'live', 'live', 'idle', 'idle']
    );
    check('sample 7 days', sample.spend.last7Days.providers.length, 3);
    check('sample projects', sample.spend.today.projects[0].project, '~/code/headroom');
    check('sample folded projects', sample.spend.today.projectsOther.count, 3);
    check('sample cost per mtok', sample.spend.today.costPerMtokMicros, 2_964_489);
}

export function testPayload() {
    testPanelItems();
    testPanelFallback();
    testAccountAdditions();
    testProviderStatus();
    testSpendAdditions();
    testLinks();
}
