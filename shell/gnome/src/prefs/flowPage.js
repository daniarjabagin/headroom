import Adw from 'gi://Adw';
import Gtk from 'gi://Gtk';
import { pillButton, providerImage, wrapLabel } from './widgets.js';

const BODY_MARGIN = 24;

export function stack(children) {
    const box = new Gtk.Box({ orientation: Gtk.Orientation.VERTICAL, spacing: 12 });
    for (const child of children) box.append(child);
    return box;
}

export function navigationPage(title, content) {
    const toolbar = new Adw.ToolbarView();
    toolbar.add_top_bar(new Adw.HeaderBar());
    toolbar.content = new Gtk.ScrolledWindow({
        hscrollbar_policy: Gtk.PolicyType.NEVER,
        propagate_natural_height: true,
        child: content,
    });
    return new Adw.NavigationPage({ title, child: toolbar });
}

export function flowBody(dir, providerId, heading) {
    const box = new Gtk.Box({ orientation: Gtk.Orientation.VERTICAL, spacing: 18 });
    for (const side of ['bottom', 'start', 'end']) box[`margin_${side}`] = BODY_MARGIN;
    box.margin_top = 6;
    const title = new Gtk.Label({ label: heading, wrap: true, justify: Gtk.Justification.CENTER });
    title.add_css_class('title-2');
    const description = wrapLabel('', ['dim-label']);
    description.justify = Gtk.Justification.CENTER;
    for (const child of [providerImage(dir, providerId, 48), title, description]) box.append(child);
    return { box, description };
}

export function pageStack() {
    return new Gtk.Stack({ transition_type: Gtk.StackTransitionType.CROSSFADE, vhomogeneous: false });
}

export function resultPage(kind, actionLabel, onAction) {
    const icon = new Gtk.Image({
        icon_name: kind === 'done' ? 'object-select-symbolic' : 'dialog-warning-symbolic',
        pixel_size: 32,
        css_classes: [kind === 'done' ? 'success' : 'warning'],
    });
    return stack([icon, pillButton(actionLabel, true, onAction)]);
}
