import Adw from 'gi://Adw';
import { _ } from '../i18n.js';
import { notificationsPatch, providerThresholdPatch } from '../settings.js';
import {
    differSubtitle,
    providerThreshold,
    providerThresholdOptions,
    thresholdChoice,
    thresholdFromChoice,
    thresholdOptions,
    thresholdProviders,
} from './notificationsModel.js';
import { comboRow, group, segmentedRow } from './rows.js';
import { providerImage } from './widgets.js';

export class ThresholdRows {
    constructor(client, dir) {
        this._client = client;
        this._dir = dir;
        this._entries = [];
        this._shape = '';
        this._threshold = segmentedRow({
            title: _('Alert when less than'),
            subtitle: _('Used by Almost out'),
            options: thresholdOptions(10),
            onChange: value => client.updateSettings(notificationsPatch({ thresholdPercent: Number(value) })),
        });
        this._perProvider = new Adw.ExpanderRow({ title: _('Per provider') });
        this.group = group(_('Alert Threshold'), [this._threshold.row, this._perProvider]);
    }

    update(settings, state) {
        const { thresholdPercent, providerThresholds } = settings.notifications;
        this._threshold.setOptions(thresholdOptions(thresholdPercent));
        this._threshold.set(String(thresholdPercent));
        const providers = thresholdProviders(state.accounts, providerThresholds, this._client.providers ?? []);
        const shape = JSON.stringify(providers);
        if (shape !== this._shape) this._rebuild(providers, shape);
        this._perProvider.subtitle = differSubtitle(providers, providerThresholds);
        providers.forEach((provider, index) => {
            const threshold = providerThreshold(providerThresholds, provider.id);
            const entry = this._entries[index];
            entry.setOptions(providerThresholdOptions(thresholdPercent, threshold));
            entry.set(thresholdChoice(threshold));
        });
    }

    _rebuild(providers, shape) {
        for (const entry of this._entries) this._perProvider.remove(entry.row);
        this._entries = providers.map(provider => this._providerRow(provider));
        for (const entry of this._entries) this._perProvider.add_row(entry.row);
        this._shape = shape;
    }

    _providerRow(provider) {
        const entry = comboRow({
            title: provider.name,
            options: providerThresholdOptions(10, null),
            onChange: choice =>
                this._client.updateSettings(providerThresholdPatch(provider.id, thresholdFromChoice(choice))),
        });
        entry.row.use_markup = false;
        entry.row.add_prefix(providerImage(this._dir, provider.id, 24));
        return entry;
    }
}
