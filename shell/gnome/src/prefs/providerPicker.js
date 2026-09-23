import Adw from 'gi://Adw';
import Gtk from 'gi://Gtk';
import { _, fill } from '../i18n.js';
import { navigationPage } from './flowPage.js';
import { methodSummary, providerSummary } from './registry.js';
import { providerImage } from './widgets.js';

const LIST_MARGIN = 18;

function choiceRow({ title, subtitle, prefix, onActivate }) {
    const row = new Adw.ActionRow({ title, activatable: true, use_markup: false });
    if (subtitle) row.subtitle = subtitle;
    if (prefix) row.add_prefix(prefix);
    row.add_suffix(new Gtk.Image({ icon_name: 'go-next-symbolic' }));
    row.connect('activated', () => onActivate());
    return row;
}

function listContent(rows, heading) {
    const box = new Gtk.Box({ orientation: Gtk.Orientation.VERTICAL, spacing: 12 });
    for (const side of ['top', 'bottom', 'start', 'end']) box[`margin_${side}`] = LIST_MARGIN;
    const intro = new Gtk.Label({ label: heading, wrap: true, xalign: 0, css_classes: ['dim-label'] });
    const list = new Gtk.ListBox({ selection_mode: Gtk.SelectionMode.NONE, css_classes: ['boxed-list'] });
    for (const row of rows) list.append(row);
    box.append(intro);
    box.append(list);
    return box;
}

function unavailablePage(error) {
    const status = new Adw.StatusPage({
        icon_name: 'dialog-warning-symbolic',
        title: _('No providers available'),
        description: error ?? _('The Headroom service did not list any providers.'),
    });
    return navigationPage(_('Add Account'), status);
}

export function providerPickerPage({ dir, providers, error, onPick }) {
    if (providers.length === 0) return unavailablePage(error);
    const rows = providers.map(provider =>
        choiceRow({
            title: provider.displayName,
            subtitle: providerSummary(provider),
            prefix: providerImage(dir, provider.id, 24),
            onActivate: () => onPick(provider),
        })
    );
    return navigationPage(_('Add Account'), listContent(rows, _('Choose the service to track.')));
}

export function methodPickerPage({ provider, onPick }) {
    const rows = provider.methods.map(method =>
        choiceRow({ title: methodSummary(method), subtitle: null, prefix: null, onActivate: () => onPick(method) })
    );
    const heading = fill(_('How do you want to add your {provider} account?'), { provider: provider.displayName });
    return navigationPage(provider.displayName, listContent(rows, heading));
}
