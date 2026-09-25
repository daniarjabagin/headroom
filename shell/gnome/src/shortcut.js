import Meta from 'gi://Meta';
import Shell from 'gi://Shell';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import { shortcutChange } from './shortcutPlan.js';

const ACTION_MODES = Shell.ActionMode.NORMAL | Shell.ActionMode.OVERVIEW | Shell.ActionMode.POPUP;
const NO_ACTION = 0;

export class GlobalShortcut {
    constructor(onActivated) {
        this._onActivated = onActivated;
        this._accelerator = '';
        this._action = NO_ACTION;
        this._activatedId = global.display.connect('accelerator-activated', (_display, action) => {
            if (action !== NO_ACTION && action === this._action) this._onActivated();
        });
    }

    set(accelerator) {
        const change = shortcutChange(this._accelerator, accelerator);
        if (change === null) return;
        if (change.release) this._release();
        this._accelerator = change.grab;
        if (change.grab !== '') this._grab(change.grab);
    }

    destroy() {
        this._release();
        this._accelerator = '';
        global.display.disconnect(this._activatedId);
    }

    _grab(accelerator) {
        const action = global.display.grab_accelerator(accelerator, Meta.KeyBindingFlags.NONE);
        if (action === NO_ACTION) {
            console.warn(`Headroom: could not register the shortcut ${accelerator}`);
            return;
        }
        this._action = action;
        Main.wm.allowKeybinding(Meta.external_binding_name_for_action(action), ACTION_MODES);
    }

    _release() {
        if (this._action === NO_ACTION) return;
        Main.wm.allowKeybinding(Meta.external_binding_name_for_action(this._action), Shell.ActionMode.NONE);
        global.display.ungrab_accelerator(this._action);
        this._action = NO_ACTION;
    }
}
