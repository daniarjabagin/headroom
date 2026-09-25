import { _ } from '../i18n.js';
import { displayPatch, supports06 } from '../settings.js';
import { group, segmentedRow, switchRow } from './rows.js';

export class AppearanceRows {
    constructor(client) {
        this._client = client;
        this._rows = { ...this._themeRows(), ...this._layoutRows(), ...this._popupRows() };
        const rows = this._rows;
        this.groups = [
            group(_('Appearance'), [
                rows.theme.row,
                rows.language.row,
                rows.timeFormat.row,
                rows.density.row,
                rows.translucent.row,
            ]),
            group(_('Popup'), [rows.valueMode.row, rows.resetFormat.row, rows.combineAccounts.row]),
        ];
    }

    update(settings, state) {
        const display = settings.display;
        for (const key of Object.keys(this._rows)) this._rows[key].set(display[key]);
        const supported = supports06(state);
        this._rows.timeFormat.row.visible = supported;
        this._rows.density.row.visible = supported;
    }

    _display(key) {
        return value => this._client.updateSettings(displayPatch({ [key]: value }));
    }

    _themeRows() {
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

    _layoutRows() {
        return {
            timeFormat: segmentedRow({
                title: _('Time format'),
                subtitle: _('Exact reset times and chart labels'),
                options: [
                    { value: 'auto', label: _('Automatic') },
                    { value: '24h', label: _('24-hour') },
                    { value: '12h', label: _('12-hour') },
                ],
                onChange: this._display('timeFormat'),
            }),
            density: segmentedRow({
                title: _('Density'),
                subtitle: _('Compact fits more accounts in the popup'),
                options: [
                    { value: 'normal', label: _('Normal') },
                    { value: 'compact', label: _('Compact') },
                ],
                onChange: this._display('density'),
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
            combineAccounts: switchRow({
                title: _('Combine accounts of the same provider'),
                subtitle: _('Show one card per provider and add up the limits of its accounts'),
                onChange: this._display('combineAccounts'),
            }),
        };
    }
}
