import Adw from 'gi://Adw';
import Gio from 'gi://Gio';
import { ExtensionPreferences } from 'resource:///org/gnome/Shell/Extensions/js/extensions/prefs.js';

function switchRow(settings, key, title, subtitle) {
    const row = new Adw.SwitchRow({ title, subtitle });
    settings.bind(key, row, 'active', Gio.SettingsBindFlags.DEFAULT);
    return row;
}

export default class HeadroomPreferences extends ExtensionPreferences {
    fillPreferencesWindow(window) {
        const settings = this.getSettings();
        const page = new Adw.PreferencesPage();
        const panel = new Adw.PreferencesGroup({ title: 'Top Panel' });
        panel.add(
            switchRow(settings, 'show-panel-percent', 'Show Percentage', 'Remaining share of the headline limit')
        );
        const usage = new Adw.PreferencesGroup({ title: 'Usage Display' });
        usage.add(
            switchRow(
                settings,
                'always-show-pacing',
                'Always Show Pacing',
                'Even-pace tick and projection on healthy limits'
            )
        );
        page.add(panel);
        page.add(usage);
        window.add(page);
    }
}
