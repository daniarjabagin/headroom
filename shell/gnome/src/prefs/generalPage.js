import Adw from 'gi://Adw';
import { _, fill, n_ } from '../i18n.js';
import { adaptiveRefreshPatch, refreshIntervalPatch, supports06 } from '../settings.js';
import { AppearanceRows } from './appearanceRows.js';
import { CardsRows } from './cardsRows.js';
import { PanelRows } from './panelRows.js';
import { PrivacyRows } from './privacyRows.js';
import { comboRow, group, switchRow } from './rows.js';
import { SpendRows } from './spendRows.js';

const REFRESH_PRESETS = [60, 120, 300, 600, 900, 1800, 3600];

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

export class GeneralPage {
    constructor(client, runner, dir) {
        this.page = new Adw.PreferencesPage({ title: _('General'), icon_name: 'preferences-system-symbolic' });
        this._appearance = new AppearanceRows(client);
        this._panel = new PanelRows(client, dir);
        this._spend = new SpendRows(client);
        this._cards = new CardsRows(client, dir);
        this._privacy = new PrivacyRows(client, runner);
        this._refresh = comboRow({
            title: _('Refresh interval'),
            subtitle: _('How often the service asks each provider'),
            options: refreshOptions(300),
            onChange: value => client.updateSettings(refreshIntervalPatch(value)),
        });
        this._adaptive = switchRow({
            title: _('Faster while coding tools run'),
            subtitle: _('Every minute while Claude Code, Codex or Cursor is open'),
            onChange: value => client.updateSettings(adaptiveRefreshPatch(value)),
        });
        const groups = [
            ...this._appearance.groups,
            this._panel.group,
            ...this._spend.groups,
            group(_('Data refresh'), [this._refresh.row, this._adaptive.row]),
            this._cards.group,
            ...this._privacy.groups,
        ];
        for (const entry of groups) this.page.add(entry);
    }

    update(settings, state) {
        this._appearance.update(settings, state);
        this._panel.update(settings, state);
        this._spend.update(settings, state);
        this._refresh.setOptions(refreshOptions(settings.refreshIntervalSecs));
        this._refresh.set(settings.refreshIntervalSecs);
        this._adaptive.set(settings.adaptiveRefresh);
        this._adaptive.row.visible = supports06(state);
        this._cards.update(settings, state);
        this._privacy.update(settings, state);
    }

    syncUpdateRun() {
        this._privacy.updates.sync();
    }
}
