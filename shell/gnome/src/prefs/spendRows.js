import { _ } from '../i18n.js';
import { displayPatch, supports06 } from '../settings.js';
import { comboRow, group, switchRow } from './rows.js';

const SECTIONS = [
    ['showAccountSpend', () => _('Per-account spend'), () => _('Today, yesterday and 30 days under each account')],
    ['showTrend', () => _('Usage trend'), () => _('Daily token bars for the last 30 days')],
    ['showForecast', () => _('Pace forecast'), () => _('Where each limit lands at the current pace')],
];

export class SpendRows {
    constructor(client) {
        this._client = client;
        this._rows = this._spendRows();
        this._sections = SECTIONS.map(([key, title, subtitle]) =>
            switchRow({ title: title(), subtitle: subtitle(), onChange: this._display(key) })
        );
        const rows = this._rows;
        this.groups = [
            group(_('Spend'), [rows.showSpend.row, rows.spendPeriod.row, rows.spendUnit.row, rows.spendBreakdown.row]),
            group(
                _('Sections'),
                this._sections.map(entry => entry.row)
            ),
        ];
    }

    update(settings, state) {
        const display = settings.display;
        for (const key of Object.keys(this._rows)) this._rows[key].set(display[key]);
        SECTIONS.forEach(([key], index) => this._sections[index].set(display[key]));
        const supported = supports06(state);
        for (const key of ['spendPeriod', 'spendUnit', 'spendBreakdown']) this._rows[key].row.visible = supported;
    }

    _display(key) {
        return value => this._client.updateSettings(displayPatch({ [key]: value }));
    }

    _spendRows() {
        return {
            showSpend: switchRow({
                title: _('Show spend'),
                subtitle: _('Spend ring for all tools at the top of the popup'),
                onChange: this._display('showSpend'),
            }),
            spendPeriod: comboRow({
                title: _('Default period'),
                subtitle: _('The tab the spend ring opens on'),
                options: [
                    { value: 'today', label: _('Today') },
                    { value: 'yesterday', label: _('Yesterday') },
                    { value: '7d', label: _('7 days') },
                    { value: '30d', label: _('30 days') },
                ],
                onChange: this._display('spendPeriod'),
            }),
            spendUnit: comboRow({
                title: _('Units'),
                options: [
                    { value: 'cost', label: _('Cost') },
                    { value: 'tokens', label: _('Tokens') },
                    { value: 'cost_per_mtok', label: _('Cost per MTok') },
                ],
                onChange: this._display('spendUnit'),
            }),
            spendBreakdown: comboRow({
                title: _('Breakdown on hover'),
                subtitle: _('Split a slice of the ring when the pointer rests on it'),
                options: [
                    { value: 'models', label: _('Models') },
                    { value: 'projects', label: _('Projects') },
                ],
                onChange: this._display('spendBreakdown'),
            }),
        };
    }
}
