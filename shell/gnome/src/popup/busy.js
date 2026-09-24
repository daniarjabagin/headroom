import Clutter from 'gi://Clutter';
import * as Animation from 'resource:///org/gnome/shell/ui/animation.js';
import { themeIcon } from '../widgets.js';

export function busyIndicator(motion, size, styleClass) {
    if (!motion.enabled) return themeIcon('view-refresh-symbolic', `${styleClass} static`);
    const spinner = new Animation.Spinner(size, { animate: true });
    spinner.add_style_class_name(styleClass);
    spinner.y_align = Clutter.ActorAlign.CENTER;
    spinner.play();
    return spinner;
}
