import Gio from 'gi://Gio';

const SCHEMA = 'org.gnome.desktop.interface';
const KEY = 'clock-format';

function interfaceSettings() {
    const schema = Gio.SettingsSchemaSource.get_default()?.lookup(SCHEMA, true);
    return schema?.has_key(KEY) ? new Gio.Settings({ settings_schema: schema }) : null;
}

export class DesktopClock {
    constructor(onChanged) {
        this._settings = interfaceSettings();
        this._changedId = this._settings?.connect(`changed::${KEY}`, () => onChanged()) ?? 0;
    }

    get format() {
        return this._settings?.get_string(KEY) === '12h' ? '12h' : '24h';
    }

    destroy() {
        if (this._changedId) this._settings.disconnect(this._changedId);
        this._changedId = 0;
        this._settings = null;
    }
}
