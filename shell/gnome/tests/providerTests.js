import {
    addAccountArgs,
    methodSummary,
    parseProviders,
    providerSummary,
    RegistryError,
} from '../src/prefs/registry.js';
import { accountName, accountTitle, iconCandidates, seriesKey, showsName } from '../src/providers.js';
import { parseState } from '../src/state.js';
import { check, throws } from './check.js';

const REGISTRY = JSON.stringify({
    version: 1,
    providers: [
        { id: 'codex', display_name: 'Codex', add_account: [{ kind: 'cli_login', program: 'codex' }] },
        {
            id: 'openrouter',
            display_name: 'OpenRouter',
            multi_account: true,
            add_account: [
                {
                    kind: 'api_key',
                    label: 'API key',
                    console_url: 'https://openrouter.ai/settings/keys',
                    hint: 'Starts with sk-or-',
                },
            ],
        },
        { id: 'cursor', display_name: 'Cursor', add_account: [{ kind: 'auto_detect', reason: 'Reads the app.' }] },
        { id: 'future', display_name: 'Future', add_account: [{ kind: 'telepathy' }] },
        { id: 'Bad Id', display_name: 'Bad', add_account: [{ kind: 'cli_login', program: 'x' }] },
        { id: 'codex', display_name: 'Codex again', add_account: [{ kind: 'cli_login', program: 'codex' }] },
        {
            id: 'kimi',
            add_account: [
                { kind: 'cli_login', program: 'kimi' },
                { kind: 'api_key', console_url: 'javascript:alert(1)' },
            ],
        },
    ],
});

function testRegistry() {
    const providers = parseProviders(REGISTRY);
    check(
        'registry ids',
        providers.map(provider => provider.id),
        ['codex', 'openrouter', 'cursor', 'kimi']
    );
    check('api key method', providers[1].methods[0], {
        kind: 'api_key',
        label: 'API key',
        consoleUrl: 'https://openrouter.ai/settings/keys',
        hint: 'Starts with sk-or-',
    });
    check('auto detect method', providers[2].methods[0], { kind: 'auto_detect', reason: 'Reads the app.' });
    check('name falls back to id', providers[3].displayName, 'kimi');
    check('only https console links', providers[3].methods[1].consoleUrl, null);
    check('multi account', [providers[0].multiAccount, providers[1].multiAccount], [false, true]);
    check('summary', providerSummary(providers[3]), 'Sign in with kimi · API key');
    check('auto summary', methodSummary(providers[2].methods[0]), 'Detected automatically');
    throws('registry bad json', () => parseProviders('{'), RegistryError);
    throws('registry version', () => parseProviders('{"version": 2, "providers": []}'), RegistryError);
    check('registry empty', parseProviders('{"version": 1}'), []);
}

function testAddArgs() {
    const apiKey = { kind: 'api_key' };
    const login = { kind: 'cli_login', program: 'codex' };
    check('api key args', addAccountArgs('grok', apiKey, ''), ['accounts', 'add', 'grok', '--api-key-stdin']);
    check('api key label', addAccountArgs('grok', apiKey, 'Work'), [
        'accounts',
        'add',
        'grok',
        '--api-key-stdin',
        '--label',
        'Work',
    ]);
    check('login args', addAccountArgs('codex', login, ''), ['accounts', 'add', 'codex']);
}

function testLookups() {
    check('known series', [seriesKey('codex'), seriesKey('claude'), seriesKey('grok')], ['codex', 'claude', 'grok']);
    check('unknown series is stable', seriesKey('newcomer'), seriesKey('newcomer'));
    check('unknown series from fallback', /^other-[0-3]$/.test(seriesKey('newcomer')), true);
    check(
        'icon candidates',
        iconCandidates('claude').map(candidate => [candidate.file, candidate.tinted]),
        [
            ['claude.svg', false],
            ['claude-symbolic.svg', true],
        ]
    );
    check('unsafe icon ids', [iconCandidates('../x'), iconCandidates(null)], [[], []]);
}

function testNames(state) {
    const [, , , , openrouter, grok] = state.accounts;
    check('provider names', [openrouter.providerName, grok.providerName], ['OpenRouter', 'Grok']);
    check('balance only', [openrouter.windows.length, openrouter.balances[0].usdMicros], [0, 7_420_000]);
    check('grok weekly', grok.windows[0].id, 'weekly');
    check('title without label', accountTitle(openrouter, showsName(openrouter, state.accounts)), 'OpenRouter');
    check('title with account', accountTitle(state.accounts[0], true), 'Codex: work');
    check(
        'name falls back to provider',
        [accountName(openrouter), accountName(grok)],
        ['OpenRouter', 'dev@example.com']
    );
    const bare = parseState('{"version": 1, "accounts": [{"id": "x:1", "provider": "x"}]}');
    check('missing provider name uses id', bare.accounts[0].providerName, 'x');
}

export function testProviders(state) {
    testRegistry();
    testAddArgs();
    testLookups();
    testNames(state);
}
