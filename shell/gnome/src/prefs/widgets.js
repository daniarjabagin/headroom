import Adw from 'gi://Adw';
import Gtk from 'gi://Gtk';
import { providerIconFile } from '../providerIcons.js';

export function providerImage(dir, provider, pixelSize) {
    return new Gtk.Image({ gicon: providerIconFile(dir, provider).gicon, pixel_size: pixelSize });
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
