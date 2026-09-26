import { setLanguage } from '../src/i18n.js';
import { maybeShowOnboarding } from '../src/onboarding.js';
import {
    clockLabel,
    clockOptions,
    differSubtitle,
    providerThreshold,
    providerThresholdOptions,
    thresholdChoice,
    thresholdFromChoice,
    thresholdOptions,
    thresholdProviders,
} from '../src/prefs/notificationsModel.js';
import { onboardingEntries } from '../src/prefs/onboardingModel.js';
import { canChoose, limitChoices, limitsAfter, limitsSubtitle, panelRowsShown } from '../src/prefs/panelModel.js';
import { loginArgs, loginMethod, signInTarget } from '../src/prefs/registry.js';
import { captureOutcome } from '../src/prefs/shortcutModel.js';
import { REPOSITORY_URL, supportCopy } from '../src/prefs/supportModel.js';
import { check } from './check.js';

function window(id, remainingPercent, hidden = false) {
    return { id, label: null, remainingPercent, hidden };
}

function account(id, provider, extra = {}) {
    return {
        id,
        provider,
        providerName: provider === 'claude' ? 'Claude' : 'Codex',
        label: null,
        email: 'dev@example.com',
        plan: 'Pro',
        status: 'fresh',
        error: null,
        recovery: null,
        hidden: false,
        windows: [window('session', 76), window('weekly', 32)],
        ...extra,
    };
}

const CLAUDE = account('claude:1', 'claude', { label: 'personal' });
const CODEX = account('codex:2', 'codex', { windows: [window('session', 50), window('weekly', 73, true)] });

function testPanelRows() {
    check('legacy daemon keeps the pin row only', panelRowsShown('several', false), {
        shows: false,
        limit: true,
        limits: false,
        indicator: false,
        label: true,
        position: false,
    });
    check('one limit', Object.values(panelRowsShown('headline', true)), [true, true, false, true, true, true]);
    check('several limits', Object.values(panelRowsShown('several', true)), [true, false, true, true, true, true]);
    check('icon only', Object.values(panelRowsShown('icon', true)), [true, false, false, false, false, true]);
}

function testLimitChoices() {
    const chosen = [{ accountId: 'codex:2', window: 'session' }];
    const choices = limitChoices([CLAUDE, CODEX, { ...CLAUDE, id: 'claude:9', hidden: true }], chosen);
    check(
        'chosen first, hidden windows and accounts skipped',
        choices.map(choice => [choice.key, choice.checked]),
        [
            ['codex:2\nsession', true],
            ['claude:1\nsession', false],
            ['claude:1\nweekly', false],
        ]
    );
    check('title', choices[1].title, 'Claude — Session');
    check('subtitle', choices[1].subtitle, 'personal · 76% left');
    const unlisted = [{ accountId: 'gone:1', window: 'weekly' }];
    check('unlisted chosen limits are not rows', limitChoices([CLAUDE], unlisted).length, 2);
}

function testLimitToggles() {
    const one = { accountId: 'a', window: 'w' };
    const two = { accountId: 'b', window: 'w' };
    const three = { accountId: 'c', window: 'w' };
    check('append keeps order', limitsAfter([one], two, true), [one, two]);
    check('uncheck removes', limitsAfter([one, two], one, false), [two]);
    check('re-check moves to the end', limitsAfter([one, two], one, true), [two, one]);
    check('at most three', limitsAfter([one, two, three], { accountId: 'd', window: 'w' }, true), [one, two, three]);
    check(
        'full list blocks unchecked rows',
        [canChoose(3, false), canChoose(3, true), canChoose(2, false)],
        [false, true, true]
    );
    check('subtitle', limitsSubtitle(2), '2 of 3 chosen · shown in this order');
    check('empty subtitle', limitsSubtitle(0), 'None chosen · the two most critical show');
}

function testThresholds() {
    check(
        'presets',
        thresholdOptions(10).map(option => option.value),
        ['5', '10', '20', '30']
    );
    check(
        'custom stored value is kept',
        thresholdOptions(15).map(option => option.label),
        ['5%', '10%', '15%', '20%', '30%']
    );
    check(
        'provider options',
        providerThresholdOptions(10, null).map(option => option.label),
        ['Default (10%)', 'Off', '5%', '10%', '20%', '30%']
    );
    check(
        'provider custom value',
        providerThresholdOptions(10, 45).map(option => option.value),
        ['default', '0', '5', '10', '20', '30', '45']
    );
    const stored = { claude: 20, copilot: 0 };
    check(
        'provider choice',
        ['claude', 'copilot', 'codex'].map(id => thresholdChoice(providerThreshold(stored, id))),
        ['20', '0', 'default']
    );
    check('choice to patch value', ['default', '0', '20'].map(thresholdFromChoice), [null, 0, 20]);
}

function testThresholdProviders() {
    const registry = [{ id: 'copilot', displayName: 'GitHub Copilot' }];
    const providers = thresholdProviders(
        [CLAUDE, CODEX, { ...CLAUDE, id: 'claude:3' }],
        { copilot: 0, x: 5 },
        registry
    );
    check('providers', providers, [
        { id: 'claude', name: 'Claude' },
        { id: 'codex', name: 'Codex' },
        { id: 'copilot', name: 'GitHub Copilot' },
        { id: 'x', name: 'x' },
    ]);
    check('differ', differSubtitle(providers, { copilot: 0, claude: 20 }), '2 providers differ from the default');
    check('differ one', differSubtitle(providers, { claude: 20 }), '1 provider differs from the default');
    check('none differ', differSubtitle(providers, {}), 'Every provider uses the default');
}

function testClockOptions() {
    const options = clockOptions('22:00', false);
    check('half-hour steps', options.length, 48);
    check('first and last', [options[0].value, options[47].value], ['00:00', '23:30']);
    check('off-grid value kept in order', clockOptions('07:45', false)[16].value, '07:45');
    check('12-hour labels', [clockLabel('22:00', true), clockLabel('00:30', true)], ['10:00 PM', '12:30 AM']);
    check('24-hour labels', clockLabel('08:00', false), '08:00');
}

function testOnboardingEntries() {
    const signedOut = account('claude:5', 'claude', {
        status: 'signed_out',
        recovery: { action: 'sign_in', accountId: 'claude:5' },
    });
    const providers = [
        { id: 'claude', displayName: 'Claude', methods: [{ kind: 'cli_login', program: 'claude' }] },
        { id: 'kimi', displayName: 'Kimi', methods: [{ kind: 'cli_login', program: 'kimi' }] },
        { id: 'copilot', displayName: 'GitHub Copilot', methods: [{ kind: 'cli_login', program: 'gh' }] },
        { id: 'grok', displayName: 'Grok', methods: [{ kind: 'api_key' }] },
    ];
    const entries = onboardingEntries([CODEX, signedOut], providers, program => program === 'kimi');
    check(
        'entries',
        entries.map(entry => [entry.key, entry.kind, entry.subtitle]),
        [
            ['codex:2', 'account', 'Signed in · Pro · dev@example.com'],
            ['claude:5', 'signed_out', 'Found, not signed in'],
            ['kimi', 'found', 'Found, not signed in'],
            ['copilot', 'missing', 'Not installed'],
        ]
    );
    check('switch follows hidden flag', entries[0].shown, true);
}

function testShortcutCapture() {
    const outcome = (keyName, hasModifiers, accelerator = null) =>
        captureOutcome({ keyName, hasModifiers, accelerator }).kind;
    check('escape cancels', outcome('Escape', false), 'cancel');
    check('backspace disables', outcome('BackSpace', false), 'disable');
    check('plain key waits', outcome('u', false, 'u'), 'wait');
    check('modifier alone waits', outcome('Super_L', true, '<Super>Super_L'), 'wait');
    check('combination sets', captureOutcome({ keyName: 'u', hasModifiers: true, accelerator: '<Super>u' }), {
        kind: 'set',
        accelerator: '<Super>u',
    });
    check('non-ascii accelerator waits', outcome('udiaeresis', true, '<Super>ü'), 'wait');
    check('modified escape is a shortcut', outcome('Escape', true, '<Control>Escape'), 'set');
}

function testLoginArgs() {
    check('cli login', loginArgs('claude:1', { kind: 'cli_login' }), ['accounts', 'login', 'claude:1']);
    check('key login', loginArgs('grok:1', { kind: 'api_key' }), ['accounts', 'login', 'grok:1', '--api-key-stdin']);
    const provider = { methods: [{ kind: 'auto_detect' }, { kind: 'api_key' }, { kind: 'cli_login' }] };
    check('cli preferred', loginMethod(provider).kind, 'cli_login');
    const claude = { id: 'claude', methods: [{ kind: 'cli_login', program: 'claude' }] };
    const expired = { ...CLAUDE, recovery: { action: 'sign_in', accountId: 'claude:7' } };
    check('sign in target', signInTarget(expired, [claude]), { provider: claude, loginId: 'claude:7' });
    check(
        'cli-owned accounts copy a command instead',
        signInTarget({ ...CLAUDE, recovery: { action: 'cli_login' } }, [claude]),
        null
    );
    check('unknown provider', signInTarget(expired, []), null);
    check('auto detect cannot sign in', loginMethod({ methods: [{ kind: 'auto_detect' }] }), null);
}

function testOnboardingLaunch() {
    const opened = [];
    const extension = { openPreferences: () => opened.push('prefs') };
    const state = { appVersion: '0.6.0' };
    const fresh = { onboarding: { completed: false } };
    check('old daemon never onboards', maybeShowOnboarding(extension, { appVersion: '0.5.1' }, fresh), false);
    check(
        'completed never onboards',
        maybeShowOnboarding(extension, state, { onboarding: { completed: true } }),
        false
    );
    check('fresh install opens preferences', maybeShowOnboarding(extension, state, fresh), true);
    check('once per session', maybeShowOnboarding(extension, state, fresh), false);
    check('opened once', opened, ['prefs']);
}

function testSupportCopy() {
    check('support url', REPOSITORY_URL, 'https://github.com/daniarjabagin/headroom');
    check('support copy en', supportCopy(), {
        group: 'Support Headroom',
        title: 'Star Headroom on GitHub',
        subtitle: "Stars help other people find it. It's free and takes a second.",
        action: 'Open GitHub',
    });
    setLanguage('ru');
    check('support copy ru', supportCopy(), {
        group: 'Поддержать Headroom',
        title: 'Поставьте звезду на GitHub',
        subtitle: 'Звёзды помогают другим найти Headroom. Это бесплатно и занимает секунду.',
        action: 'Открыть GitHub',
    });
    setLanguage('en');
}

export function testPrefsModels() {
    setLanguage('en');
    testPanelRows();
    testLimitChoices();
    testLimitToggles();
    testThresholds();
    testThresholdProviders();
    testClockOptions();
    testOnboardingEntries();
    testShortcutCapture();
    testLoginArgs();
    testOnboardingLaunch();
    testSupportCopy();
}
