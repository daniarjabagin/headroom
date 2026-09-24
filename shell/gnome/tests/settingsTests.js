import * as settings from '../src/settings.js';
import { check, throws } from './check.js';

const SAMPLE = JSON.stringify({
    refresh_interval_secs: 120,
    notifications: { almost_out: false },
    headline: { mode: 'pinned', account_id: 'codex:1', window: 'weekly' },
    show_usage: false,
    display: {
        theme: 'dark',
        language: 'ru',
        value_mode: 'used',
        hidden_windows: { 'codex:1': ['session', 'session', ''], 'claude:2': [] },
    },
});

function testParse() {
    const parsed = settings.parseSettings(SAMPLE);
    check('settings interval', parsed.refreshIntervalSecs, 120);
    check('settings notifications', parsed.notifications, {
        almostOut: false,
        cuttingItClose: true,
        willRunOut: true,
        reset: false,
    });
    check('settings pinned', parsed.headline, { mode: 'pinned', accountId: 'codex:1', window: 'weekly' });
    const { theme, valueMode, language, resetFormat } = parsed.display;
    check('settings display', [theme, valueMode, language, resetFormat], ['dark', 'used', 'ru', 'countdown']);
    check('settings hidden windows', parsed.display.hiddenWindows, { 'codex:1': ['session'] });
    check('settings defaults', settings.parseSettings('{}').display, settings.parseDisplay(null));
    check('settings bad pin', settings.parseSettings('{"headline":{"mode":"pinned"}}').headline, { mode: 'auto' });
    check('settings clamp', settings.parseSettings('{"refresh_interval_secs":5}').refreshIntervalSecs, 60);
    check('settings bad theme', settings.parseDisplay({ theme: 'sepia' }).theme, 'system');
    check('settings translucent', settings.parseDisplay({ translucent: true }).translucent, true);
    check('settings translucent missing', settings.parseDisplay({}).translucent, false);
    check('settings translucent invalid', settings.parseDisplay({ translucent: 'yes' }).translucent, false);
    check('settings updates default', settings.parseSettings('{}').updates, { check: true });
    check('settings updates off', settings.parseSettings('{"updates":{"check":false}}').updates, { check: false });
    check('settings updates invalid', settings.parseSettings('{"updates":{"check":"no"}}').updates, { check: true });
    check('updates patch', settings.updatesPatch(false), { updates: { check: false } });
    throws('settings json', () => settings.parseSettings('['), settings.SettingsError);
    throws('settings array', () => settings.parseSettings('[]'), settings.SettingsError);
}

function testPatches() {
    check('patch display', settings.displayPatch({ valueMode: 'used' }), { display: { value_mode: 'used' } });
    check('patch display many', settings.displayPatch({ showTrend: false, translucent: true }), {
        display: { show_trend: false, translucent: true },
    });
    throws('patch unknown', () => settings.displayPatch({ sepia: true }), settings.SettingsError);
    check('patch notifications', settings.notificationsPatch({ cuttingItClose: false }), {
        notifications: { cutting_it_close: false },
    });
    check('patch interval', settings.refreshIntervalPatch(9000), { refresh_interval_secs: 3600 });
    check('patch pinned', settings.headlinePatch({ mode: 'pinned', accountId: 'codex:1', window: 'weekly' }), {
        headline: { mode: 'pinned', account_id: 'codex:1', window: 'weekly' },
    });
    check('patch auto clears pin', settings.headlinePatch({ mode: 'auto' }), {
        headline: { mode: 'auto', account_id: null, window: null },
    });
    check('patch hidden windows', settings.hiddenWindowsPatch('codex:x', ['weekly']), {
        display: { hidden_windows: { 'codex:x': ['weekly'] } },
    });
    check('patch hidden windows clear', settings.hiddenWindowsPatch('codex:x', []), {
        display: { hidden_windows: { 'codex:x': null } },
    });
}

function testMergePatch() {
    const raw = JSON.parse(SAMPLE);
    const merged = settings.mergePatch(raw, settings.displayPatch({ theme: 'light' }));
    check('merge keeps unknown', merged.show_usage, false);
    check('merge keeps siblings', [merged.display.theme, merged.display.value_mode], ['light', 'used']);
    check('merge leaves input', raw.display.theme, 'dark');
    const cleared = settings.mergePatch(raw, settings.hiddenWindowsPatch('codex:1', []));
    check('merge deletes entry', cleared.display.hidden_windows, { 'claude:2': [] });
    const auto = settings.mergePatch(raw, settings.headlinePatch({ mode: 'auto' }));
    check('merge auto headline', auto.headline, { mode: 'auto' });
    check('merge replaces arrays', settings.mergePatch({ a: [1, 2] }, { a: [3] }), { a: [3] });
    check('merge scalar target', settings.mergePatch(5, { a: { b: null, c: 1 } }), { a: { c: 1 } });
    check('merge parsed', settings.settingsFrom(merged).display.theme, 'light');
}

export function testSettings() {
    testParse();
    testPatches();
    testMergePatch();
}

export function testSettingsUpdates() {
    const display = settings.parseDisplay({ value_mode: 'left', reset_format: 'exact' });
    check('toggle value', settings.toggledValueMode(display), { valueMode: 'used' });
    check('toggle value back', settings.toggledValueMode({ valueMode: 'used' }), { valueMode: 'left' });
    check('toggle reset', settings.toggledResetFormat(display), { resetFormat: 'countdown' });
    check('toggle reset back', settings.toggledResetFormat({ resetFormat: 'countdown' }), { resetFormat: 'exact' });
    const hidden = settings.hiddenWindowsAfter(display, 'claude:1', 'weekly', true);
    check('hide window', hidden, ['weekly']);
    const withHidden = { ...display, hiddenWindows: { 'claude:1': hidden } };
    check('is hidden', settings.isWindowHidden(withHidden, 'claude:1', 'weekly'), true);
    check('is not hidden', settings.isWindowHidden(withHidden, 'claude:1', 'session'), false);
    check('show window', settings.hiddenWindowsAfter(withHidden, 'claude:1', 'weekly', false), []);
    check('hide second', settings.hiddenWindowsAfter(withHidden, 'claude:1', 'session', true), ['weekly', 'session']);
}
