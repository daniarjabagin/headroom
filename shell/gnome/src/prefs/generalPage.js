import Adw from 'gi://Adw';
import { windowLabel } from '../format.js';
import { _, fill, n_ } from '../i18n.js';
import { accountTitle, showsName } from '../providers.js';
import { withDisplay } from '../settings.js';
import { comboRow, group, segmentedRow, switchRow } from './rows.js';

const AUTO = 'auto';
const PIN_SEPARATOR = '\n';
const REFRESH_PRESETS = [60, 120, 300, 600, 900, 1800, 3600];

const SECTIONS = [
    ['showSpend', () => _('Total spend'), () => _('Spend ring for all tools at the top')],
    ['showAccountSpend', () => _('Per-account spend'), () => _('Today, yesterday and 30 days under each account')],
    ['showTrend', () => _('Usage trend'), () => _('Daily token bars for the last 30 days')],
    ['showForecast', () => _('Pace forecast'), () => _('Where each limit lands at the current pace')],
];

function refreshLabel(seconds) {
    const minutes = seconds / 60;
    if (!Number.isInteger(minutes))
        return fill(n_('Every {seconds} second', 'Every {seconds} seconds', seconds), { seconds });
    if (minutes === 1) return _('Every minute');
    return fill(n_('Every {minutes} minute', 'Every {minutes} minutes', minutes), { minutes });
}

function refreshOptions(current) {
    const values = REFRESH_PRESETS.includes(current) ? REFRESH_PRESETS : [...REFRESH_PRESETS, current];
    return values.sort((a, b) => a - b).map(value => ({ value, label: refreshLabel(value) }));
}

function pinKey(accountId, windowId) {
    return `${accountId}${PIN_SEPARATOR}${windowId}`;
}

function limitOptions(state, headline) {
    const accounts = (state?.accounts ?? []).filter(account => !account.hidden);
    const options = [{ value: AUTO, label: _('Auto — most critical') }];
    for (const account of accounts) {
        const title = accountTitle(account, showsName(account, accounts));
        for (const window of account.windows)
            options.push({
                value: pinKey(account.id, window.id),
                label: `${title} — ${windowLabel(window.id, window.label)}`,
            });
    }
    const pinned = headline.mode === 'pinned' ? pinKey(headline.accountId, headline.window) : null;
    if (pinned && !options.some(option => option.value === pinned))
        options.push({ value: pinned, label: _('Pinned limit (not available now)') });
    return options;
}

function headlineFor(value) {
    if (value === AUTO) return { mode: 'auto' };
    const [accountId, window] = value.split(PIN_SEPARATOR);
    return { mode: 'pinned', accountId, window };
}

export class GeneralPage {
    constructor(client) {
        this._client = client;
        this._limitKey = null;
        this.page = new Adw.PreferencesPage({ title: _('General'), icon_name: 'preferences-system-symbolic' });
        this._rows = this._buildRows();
        this.page.add(
            group(_('Appearance'), [this._rows.theme.row, this._rows.language.row, this._rows.translucent.row])
        );
        this.page.add(group(_('Popup'), [this._rows.valueMode.row, this._rows.resetFormat.row]));
        this.page.add(group(_('Top Panel'), [this._rows.limit.row, this._rows.panelLabel.row]));
        this.page.add(
            group(
                _('Sections'),
                this._rows.sections.map(entry => entry.row)
            )
        );
        this.page.add(group(_('Updates'), [this._rows.refresh.row]));
    }

    update(settings, state) {
        const display = settings.display;
        this._rows.theme.set(display.theme);
        this._rows.language.set(display.language);
        this._rows.translucent.set(display.translucent);
        this._rows.valueMode.set(display.valueMode);
        this._rows.resetFormat.set(display.resetFormat);
        this._rows.panelLabel.set(display.panelLabel);
        SECTIONS.forEach(([key], index) => this._rows.sections[index].set(display[key]));
        this._rows.refresh.setOptions(refreshOptions(settings.refreshIntervalSecs));
        this._rows.refresh.set(settings.refreshIntervalSecs);
        this._updateLimit(settings.headline, state);
    }

    _updateLimit(headline, state) {
        const options = limitOptions(state, headline);
        const key = JSON.stringify(options);
        if (key !== this._limitKey) this._rows.limit.setOptions(options);
        this._limitKey = key;
        this._rows.limit.set(headline.mode === 'pinned' ? pinKey(headline.accountId, headline.window) : AUTO);
    }

    _display(key) {
        return value => this._client.updateSettings(settings => withDisplay(settings, { [key]: value }));
    }

    _buildRows() {
        return {
            ...this._appearanceRows(),
            ...this._popupRows(),
            ...this._panelRows(),
            sections: SECTIONS.map(([key, title, subtitle]) =>
                switchRow({ title: title(), subtitle: subtitle(), onChange: this._display(key) })
            ),
            refresh: comboRow({
                title: _('Refresh interval'),
                subtitle: _('How often the service asks each provider'),
                options: refreshOptions(300),
                onChange: value =>
                    this._client.updateSettings(settings => ({ ...settings, refreshIntervalSecs: value })),
            }),
        };
    }

    _appearanceRows() {
        return {
            theme: segmentedRow({
                title: _('Theme'),
                options: [
                    { value: 'system', label: _('System') },
                    { value: 'light', label: _('Light') },
                    { value: 'dark', label: _('Dark') },
                ],
                onChange: this._display('theme'),
            }),
            language: segmentedRow({
                title: _('Language'),
                options: [
                    { value: 'system', label: _('System') },
                    { value: 'en', label: 'English' },
                    { value: 'ru', label: 'Русский' },
                ],
                onChange: this._display('language'),
            }),
            translucent: switchRow({
                title: _('Translucent background'),
                subtitle: _('Blur what is behind the popup'),
                onChange: this._display('translucent'),
            }),
        };
    }

    _popupRows() {
        return {
            valueMode: segmentedRow({
                title: _('Show values as'),
                subtitle: _('Click a reading in the popup to switch'),
                options: [
                    { value: 'left', label: _('Left') },
                    { value: 'used', label: _('Used') },
                ],
                onChange: this._display('valueMode'),
            }),
            resetFormat: segmentedRow({
                title: _('Reset time'),
                subtitle: _('Click a reset time in the popup to switch'),
                options: [
                    { value: 'countdown', label: _('Countdown') },
                    { value: 'exact', label: _('Exact time') },
                ],
                onChange: this._display('resetFormat'),
            }),
        };
    }

    _panelRows() {
        return {
            limit: comboRow({
                title: _('Panel limit'),
                subtitle: _('The limit shown next to the clock'),
                options: [{ value: AUTO, label: _('Auto — most critical') }],
                onChange: value =>
                    this._client.updateSettings(settings => ({ ...settings, headline: headlineFor(value) })),
            }),
            panelLabel: segmentedRow({
                title: _('Panel label'),
                options: [
                    { value: 'percent', label: _('Percent') },
                    { value: 'window', label: _('Provider + limit') },
                ],
                onChange: this._display('panelLabel'),
            }),
        };
    }
}
