import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';
import { Indicator } from './src/indicator.js';

export default class HeadroomExtension extends Extension {
    enable() {
        this._indicator = new Indicator(this);
        this._indicator.addToPanel(Main.panel, this.uuid);
    }

    disable() {
        this._indicator.destroy();
        this._indicator = null;
    }
}
