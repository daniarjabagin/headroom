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
    throws('settings json', () => settings.parseSettings('['), settings.SettingsError);
    throws('settings array', () => settings.parseSettings('[]'), settings.SettingsError);
}

function testSerialize() {
    const parsed = settings.parseSettings(SAMPLE);
    const serialized = JSON.parse(settings.serializeSettings(parsed));
    check('serialize keys', Object.keys(serialized).sort(), [
        'display',
        'headline',
        'notifications',
        'reduced_motion',
        'refresh_interval_secs',
    ]);
    check('serialize headline', serialized.headline, { mode: 'pinned', account_id: 'codex:1', window: 'weekly' });
    check('serialize auto', JSON.parse(settings.serializeSettings(settings.parseSettings('{}'))).headline, {
        mode: 'auto',
    });
    check('serialize display', serialized.display, {
        theme: 'dark',
        language: 'ru',
        value_mode: 'used',
        reset_format: 'countdown',
        panel_label: 'percent',
        show_spend: true,
        show_account_spend: true,
        show_trend: true,
        show_forecast: true,
        translucent: false,
        hidden_windows: { 'codex:1': ['session'] },
    });
    check('round trip', settings.parseSettings(settings.serializeSettings(parsed)), parsed);
}

export function testSettings() {
    testParse();
    testSerialize();
}

export function testSettingsUpdates() {
    const display = settings.parseDisplay({ value_mode: 'left', reset_format: 'exact' });
    check('toggle value', settings.toggledValueMode(display), { valueMode: 'used' });
    check('toggle value back', settings.toggledValueMode({ valueMode: 'used' }), { valueMode: 'left' });
    check('toggle reset', settings.toggledResetFormat(display), { resetFormat: 'countdown' });
    check('toggle reset back', settings.toggledResetFormat({ resetFormat: 'countdown' }), { resetFormat: 'exact' });
    const hidden = settings.withWindowHidden(display, 'claude:1', 'weekly', true);
    check('hide window', hidden, { hiddenWindows: { 'claude:1': ['weekly'] } });
    const withHidden = { ...display, ...hidden };
    check('is hidden', settings.isWindowHidden(withHidden, 'claude:1', 'weekly'), true);
    check('is not hidden', settings.isWindowHidden(withHidden, 'claude:1', 'session'), false);
    check('show window', settings.withWindowHidden(withHidden, 'claude:1', 'weekly', false), { hiddenWindows: {} });
    const base = settings.parseSettings('{}');
    check('with display', settings.withDisplay(base, { theme: 'light' }).display.theme, 'light');
    check('with display keeps', settings.withDisplay(base, { theme: 'light' }).display.showTrend, true);
    check('with notifications', settings.withNotifications(base, { reset: true }).notifications.reset, true);
}
