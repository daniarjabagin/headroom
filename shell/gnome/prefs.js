import { ExtensionPreferences } from 'resource:///org/gnome/Shell/Extensions/js/extensions/prefs.js';
import { PrefsController } from './src/prefs/controller.js';

export default class HeadroomPreferences extends ExtensionPreferences {
    fillPreferencesWindow(window) {
        const controller = new PrefsController(window, this.dir);
        window.connect('close-request', () => {
            controller.destroy();
            return false;
        });
    }
}
