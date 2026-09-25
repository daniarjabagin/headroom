import { windowLabel } from '../format.js';
import { _, C_ } from '../i18n.js';
import { accountTitle, showsName } from '../providers.js';
import { displayPatch, headlinePatch, panelPositionPatch, supports06 } from '../settings.js';
import { LimitRows } from './limitRows.js';
import { limitKey, panelRowsShown } from './panelModel.js';
import { comboRow, group, segmentedRow } from './rows.js';

const AUTO = 'auto';

function pinOptions(state, headline) {
    const accounts = (state?.accounts ?? []).filter(account => !account.hidden);
    const options = [{ value: AUTO, label: _('Auto — most critical') }];
    for (const account of accounts) {
        const title = accountTitle(account, showsName(account, accounts));
        for (const window of account.windows)
            options.push({
                value: limitKey(account.id, window.id),
                label: `${title} — ${windowLabel(window.id, window.label)}`,
            });
    }
    const pinned = headline.mode === 'pinned' ? limitKey(headline.accountId, headline.window) : null;
    if (pinned && !options.some(option => option.value === pinned))
        options.push({ value: pinned, label: _('Pinned limit (not available now)') });
    return options;
}

function headlineFor(value) {
    if (value === AUTO) return { mode: 'auto' };
    const [accountId, window] = value.split('\n');
    return { mode: 'pinned', accountId, window };
}

function labelOptions(supported) {
    const options = [
        { value: 'percent', label: _('Percent') },
        { value: 'window', label: _('Provider + limit') },
    ];
    return supported ? [...options, { value: 'none', label: _('None') }] : options;
}

export class PanelRows {
    constructor(client, dir) {
        this._client = client;
        this._position = { box: 'right', index: 0 };
        this._limits = new LimitRows(client, dir);
        this._rows = { ...this._modeRows(), ...this._styleRows() };
        this.group = group(_('Top Panel'), [
            this._rows.shows.row,
            this._rows.limit.row,
            this._limits.row,
            this._rows.indicator.row,
            this._rows.label.row,
            this._rows.position.row,
        ]);
    }

    update(settings, state) {
        const display = settings.display;
        const supported = supports06(state);
        const shown = panelRowsShown(display.panelMode, supported);
        this._rows.shows.set(display.panelMode);
        this._rows.limit.setOptions(pinOptions(state, settings.headline));
        const headline = settings.headline;
        this._rows.limit.set(headline.mode === 'pinned' ? limitKey(headline.accountId, headline.window) : AUTO);
        this._rows.indicator.set(display.panelIndicator);
        this._rows.label.setOptions(labelOptions(supported));
        this._rows.label.set(display.panelLabel);
        this._position = display.panelPosition;
        this._rows.position.set(display.panelPosition.box);
        if (shown.limits) this._limits.update(settings, state);
        this._rows.shows.row.visible = shown.shows;
        this._rows.limit.row.visible = shown.limit;
        this._limits.row.visible = shown.limits;
        this._rows.indicator.row.visible = shown.indicator;
        this._rows.label.row.visible = shown.label;
        this._rows.position.row.visible = shown.position;
    }

    _display(key) {
        return value => this._client.updateSettings(displayPatch({ [key]: value }));
    }

    _modeRows() {
        return {
            shows: comboRow({
                title: _('Panel shows'),
                subtitle: _('What sits next to the clock'),
                options: [
                    { value: 'headline', label: _('One limit') },
                    { value: 'several', label: _('Several limits') },
                    { value: 'icon', label: _('Icon only') },
                ],
                onChange: this._display('panelMode'),
            }),
            limit: comboRow({
                title: _('Panel limit'),
                subtitle: _('The limit shown next to the clock'),
                options: [{ value: AUTO, label: _('Auto — most critical') }],
                onChange: value => this._client.updateSettings(headlinePatch(headlineFor(value))),
            }),
        };
    }

    _styleRows() {
        return {
            indicator: segmentedRow({
                title: _('Indicator style'),
                subtitle: _('The mark in front of each figure'),
                options: [
                    { value: 'ring', label: _('Ring') },
                    { value: 'bar', label: _('Bar') },
                    { value: 'none', label: _('None') },
                ],
                onChange: this._display('panelIndicator'),
            }),
            label: segmentedRow({
                title: _('Panel label'),
                options: labelOptions(false),
                onChange: this._display('panelLabel'),
            }),
            position: segmentedRow({
                title: C_('panel position', 'Position'),
                subtitle: _('Or drag the indicator along the top bar'),
                options: [
                    { value: 'left', label: C_('panel position', 'Left') },
                    { value: 'center', label: C_('panel position', 'Center') },
                    { value: 'right', label: C_('panel position', 'Right') },
                ],
                onChange: box => this._movePanel(box),
            }),
        };
    }

    _movePanel(box) {
        if (box === this._position.box) return;
        this._client.updateSettings(panelPositionPatch({ box, index: 0 }));
    }
}
