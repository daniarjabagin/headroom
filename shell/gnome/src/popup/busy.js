import Clutter from 'gi://Clutter';
import * as Animation from 'resource:///org/gnome/shell/ui/animation.js';
import { themeIcon } from '../widgets.js';

function playWhileMapped(spinner) {
    spinner.connect('notify::mapped', () => {
        if (spinner.mapped) spinner.play();
        else spinner.stop();
    });
}

export function busyIndicator(motion, size, styleClass) {
    if (!motion.enabled) return themeIcon('view-refresh-symbolic', `${styleClass} static`);
    const spinner = new Animation.Spinner(size);
    spinner.add_style_class_name(styleClass);
    spinner.y_align = Clutter.ActorAlign.CENTER;
    playWhileMapped(spinner);
    return spinner;
}
