import * as settings from '../src/settings.js';
import { check, throws } from './check.js';

const FULL = {
    adaptive_refresh: false,
    notifications: {
        threshold_percent: 20,
        provider_thresholds: { claude: 20, copilot: 0, grok: 51, '': 5, cline: 'x' },
        quiet_hours: { enabled: true, from: '23:30', to: '07:00', allow_critical: false },
    },
    display: {
        density: 'compact',
        time_format: '12h',
        panel_mode: 'several',
        panel_indicator: 'bar',
        panel_label: 'none',
        panel_limits: [
            { account_id: 'claude:1', window: 'session' },
            { account_id: 'claude:1', window: 'session' },
            { account_id: 'codex:2', window: 'weekly' },
            { account_id: '', window: 'weekly' },
            { account_id: 'grok:3', window: 'weekly' },
            { account_id: 'kimi:4', window: 'weekly' },
        ],
        panel_position: { box: 'left', index: 2 },
        spend_period: '7d',
        spend_unit: 'cost_per_mtok',
        spend_breakdown: 'projects',
        starred_accounts: ['claude:1', 'claude:1', '', 'codex:2'],
        collapse_unstarred: true,
        hide_on_screen_share: false,
    },
    status_pages: { enabled: true },
    shortcuts: { open: '<Super>u' },
    logging: { level: 'debug' },
    onboarding: { completed: true },
};

function testDefaults() {
    const parsed = settings.parseSettings('{}');
    check('06 adaptive default', parsed.adaptiveRefresh, true);
    check('06 status pages default', parsed.statusPages, { enabled: false });
    check('06 shortcut default', parsed.shortcuts, { open: '' });
    check('06 logging default', parsed.logging, { level: 'info' });
    check('06 onboarding default', parsed.onboarding, { completed: false });
    const { display } = parsed;
    check(
        '06 display defaults',
        [display.density, display.timeFormat, display.panelMode, display.panelIndicator, display.panelLabel],
        ['normal', 'auto', 'headline', 'ring', 'percent']
    );
    check('06 panel position default', display.panelPosition, { box: 'right', index: 0 });
    check(
        '06 spend defaults',
        [display.spendPeriod, display.spendUnit, display.spendBreakdown],
        ['30d', 'cost', 'models']
    );
    check(
        '06 card defaults',
        [display.panelLimits, display.starredAccounts, display.collapseUnstarred, display.hideOnScreenShare],
        [[], [], false, true]
    );
}

function testFullDocument() {
    const parsed = settings.settingsFrom(FULL);
    check('06 adaptive', parsed.adaptiveRefresh, false);
    check('06 threshold', parsed.notifications.thresholdPercent, 20);
    check('06 provider thresholds', parsed.notifications.providerThresholds, { claude: 20, copilot: 0 });
    check('06 quiet hours', parsed.notifications.quietHours, {
        enabled: true,
        from: '23:30',
        to: '07:00',
        allowCritical: false,
    });
    check('06 panel limits', parsed.display.panelLimits, [
        { accountId: 'claude:1', window: 'session' },
        { accountId: 'codex:2', window: 'weekly' },
        { accountId: 'grok:3', window: 'weekly' },
    ]);
    check('06 panel position', parsed.display.panelPosition, { box: 'left', index: 2 });
    check('06 panel label none', parsed.display.panelLabel, 'none');
    check('06 starred', parsed.display.starredAccounts, ['claude:1', 'codex:2']);
    check(
        '06 spend',
        [parsed.display.spendPeriod, parsed.display.spendUnit, parsed.display.spendBreakdown],
        ['7d', 'cost_per_mtok', 'projects']
    );
    check('06 shortcut', parsed.shortcuts.open, '<Super>u');
    check('06 logging', parsed.logging.level, 'debug');
    check('06 onboarding', parsed.onboarding.completed, true);
}

function testMalformed() {
    const bad = settings.settingsFrom({
        adaptive_refresh: 'yes',
        notifications: { threshold_percent: 0, quiet_hours: { enabled: true, from: '24:00', to: '22:00' } },
        display: {
            density: 'tiny',
            time_format: 'military',
            panel_limits: 'claude',
            panel_position: { box: 'top', index: 1 },
            starred_accounts: 'claude:1',
        },
        shortcuts: { open: 'Super+U' },
        logging: { level: 'trace' },
    });
    check('06 bad adaptive', bad.adaptiveRefresh, true);
    check('06 bad threshold', bad.notifications.thresholdPercent, 10);
    check('06 quiet hours same times disable', bad.notifications.quietHours.enabled, false);
    check('06 bad clock', bad.notifications.quietHours.from, '22:00');
    check('06 bad density', bad.display.density, 'normal');
    check('06 bad time format', bad.display.timeFormat, 'auto');
    check('06 bad limits', bad.display.panelLimits, []);
    check('06 bad position', bad.display.panelPosition, { box: 'right', index: 0 });
    check('06 negative index', settings.parseDisplay({ panel_position: { box: 'left', index: -1 } }).panelPosition, {
        box: 'right',
        index: 0,
    });
    check('06 bad starred', bad.display.starredAccounts, []);
    check('06 bad shortcut', bad.shortcuts.open, '');
    check('06 bad logging', bad.logging.level, 'info');
    check('06 accelerators', ['<Control><Alt>h', 'F12', '', '<Super>', 'a b'].map(settings.isAccelerator), [
        true,
        true,
        true,
        false,
        false,
    ]);
    check('06 long accelerator', settings.isAccelerator(`<Super>${'a'.repeat(60)}`), false);
}

function testPatches() {
    check('patch density', settings.displayPatch({ density: 'compact', timeFormat: '24h' }), {
        display: { density: 'compact', time_format: '24h' },
    });
    throws('patch bad density', () => settings.displayPatch({ density: 'tiny' }), settings.SettingsError);
    throws('patch bad flag', () => settings.displayPatch({ collapseUnstarred: 'yes' }), settings.SettingsError);
    check('patch threshold', settings.notificationsPatch({ thresholdPercent: 30 }), {
        notifications: { threshold_percent: 30 },
    });
    throws('patch threshold range', () => settings.notificationsPatch({ thresholdPercent: 0 }), settings.SettingsError);
    check('patch provider off', settings.providerThresholdPatch('copilot', 0), {
        notifications: { provider_thresholds: { copilot: 0 } },
    });
    check('patch provider default', settings.providerThresholdPatch('claude', null), {
        notifications: { provider_thresholds: { claude: null } },
    });
    throws('patch provider range', () => settings.providerThresholdPatch('claude', 51), settings.SettingsError);
    throws('patch provider empty', () => settings.providerThresholdPatch('', 5), settings.SettingsError);
    check('patch quiet hours', settings.quietHoursPatch({ enabled: true, from: '21:00', allowCritical: false }), {
        notifications: { quiet_hours: { enabled: true, from: '21:00', allow_critical: false } },
    });
    throws('patch quiet clock', () => settings.quietHoursPatch({ to: '7:00' }), settings.SettingsError);
}

function testPanelPatches() {
    check(
        'patch limits',
        settings.panelLimitsPatch([
            { accountId: 'claude:1', window: 'session' },
            { accountId: 'claude:1', window: 'session' },
            { accountId: 'codex:2', window: 'weekly' },
        ]),
        {
            display: {
                panel_limits: [
                    { account_id: 'claude:1', window: 'session' },
                    { account_id: 'codex:2', window: 'weekly' },
                ],
            },
        }
    );
    const four = ['a', 'b', 'c', 'd'].map(id => ({ accountId: `claude:${id}`, window: 'session' }));
    throws('patch too many limits', () => settings.panelLimitsPatch(four), settings.SettingsError);
    throws('patch empty limit', () => settings.panelLimitsPatch([{ accountId: 'x' }]), settings.SettingsError);
    check('patch position', settings.panelPositionPatch({ box: 'center', index: 1 }), {
        display: { panel_position: { box: 'center', index: 1 } },
    });
    throws('patch bad position', () => settings.panelPositionPatch({ box: 'top', index: 0 }), settings.SettingsError);
    check('patch starred', settings.starredAccountsPatch(['a', 'a', '', 'b']), {
        display: { starred_accounts: ['a', 'b'] },
    });
}

function testTopLevelPatches() {
    check('patch shortcut', settings.shortcutPatch('<Super>u'), { shortcuts: { open: '<Super>u' } });
    throws('patch bad shortcut', () => settings.shortcutPatch('Super+U'), settings.SettingsError);
    check('patch logging', settings.loggingPatch('warn'), { logging: { level: 'warn' } });
    throws('patch bad logging', () => settings.loggingPatch('trace'), settings.SettingsError);
    check(
        'patch toggles',
        [settings.adaptiveRefreshPatch(false), settings.statusPagesPatch(true), settings.onboardingPatch(true)],
        [{ adaptive_refresh: false }, { status_pages: { enabled: true } }, { onboarding: { completed: true } }]
    );
}

function testStars() {
    const display = settings.parseDisplay({ starred_accounts: ['claude:1'] });
    check('is starred', settings.isStarred(display, 'claude:1'), true);
    check('star second', settings.starredAfter(display, 'codex:2', true), ['claude:1', 'codex:2']);
    check('unstar', settings.starredAfter(display, 'claude:1', false), []);
}

function testGating() {
    check('06 needs: display key', settings.needsSettings06(settings.displayPatch({ density: 'compact' })), true);
    check('06 needs: panel label none', settings.needsSettings06(settings.displayPatch({ panelLabel: 'none' })), true);
    check('06 needs: old label', settings.needsSettings06(settings.displayPatch({ panelLabel: 'window' })), false);
    check('06 needs: old theme', settings.needsSettings06(settings.displayPatch({ theme: 'dark' })), false);
    check('06 needs: threshold', settings.needsSettings06(settings.providerThresholdPatch('claude', 5)), true);
    check('06 needs: old notification', settings.needsSettings06(settings.notificationsPatch({ reset: true })), false);
    check('06 needs: top level', settings.needsSettings06(settings.loggingPatch('warn')), true);
    check('06 needs: interval', settings.needsSettings06(settings.refreshIntervalPatch(120)), false);
    check('release compare', settings.isReleaseAtLeast('0.6.0', [0, 6, 0]), true);
    check('release newer', settings.isReleaseAtLeast('1.0.0-beta.1', [0, 6, 0]), true);
    check('release older', settings.isReleaseAtLeast('0.5.9', [0, 6, 0]), false);
    check('release garbage', settings.isReleaseAtLeast('dev', [0, 6, 0]), false);
    check('supports 0.6', settings.supports06({ appVersion: '0.6.1', reportsPanelItems: false }), true);
    check(
        'supports 0.5.1 with panel items',
        settings.supports06({ appVersion: '0.5.1', reportsPanelItems: true }),
        true
    );
    check('no 0.5.1', settings.supports06({ appVersion: '0.5.1', reportsPanelItems: false }), false);
    check('no version', settings.supports06({ appVersion: null, reportsPanelItems: false }), false);
    check('no state', settings.supports06(null), false);
    const fresh = settings.parseSettings('{}');
    const capable = { appVersion: '0.6.0', reportsPanelItems: true };
    check('onboarding on 0.6', settings.needsOnboarding(capable, fresh), true);
    check('onboarding done', settings.needsOnboarding(capable, settings.settingsFrom(FULL)), false);
    check('no onboarding on old daemon', settings.needsOnboarding({ appVersion: '0.5.0' }, fresh), false);
}

export function testSettings06() {
    testDefaults();
    testFullDocument();
    testMalformed();
    testPatches();
    testPanelPatches();
    testTopLevelPatches();
    testStars();
    testGating();
}
