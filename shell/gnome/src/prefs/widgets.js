import Adw from 'gi://Adw';
import Gio from 'gi://Gio';
import Gtk from 'gi://Gtk';
import { providerInfo } from '../providers.js';

export function providerImage(dir, provider, pixelSize) {
    const info = providerInfo(provider);
    if (info.icon === null)
        return new Gtk.Image({ icon_name: 'application-x-executable-symbolic', pixel_size: pixelSize });
    const gicon = new Gio.FileIcon({ file: dir.get_child('icons').get_child(info.icon) });
    return new Gtk.Image({ gicon, pixel_size: pixelSize });
}

export function pillButton(label, suggested, onClick) {
    const button = new Gtk.Button({ label, halign: Gtk.Align.CENTER, css_classes: ['pill'] });
    if (suggested) button.add_css_class('suggested-action');
    button.connect('clicked', () => onClick());
    return button;
}

export function spinner() {
    if (Adw.Spinner) return new Adw.Spinner();
    return new Gtk.Spinner({ spinning: true });
}

export function wrapLabel(label, cssClasses = []) {
    return new Gtk.Label({ label, wrap: true, css_classes: cssClasses, max_width_chars: 48 });
}
