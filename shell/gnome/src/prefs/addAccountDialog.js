import Adw from 'gi://Adw';
import { ApiKeyPage } from './apiKeyPage.js';
import { AutoDetectPage } from './autoDetectPage.js';
import { CliLoginPage } from './cliLoginPage.js';
import { methodPickerPage, providerPickerPage } from './providerPicker.js';

const CONTENT_WIDTH = 460;
const CONTENT_HEIGHT = 600;

export class AddAccountDialog {
    constructor({ dir, providers, providersError, onRescan }) {
        this._dir = dir;
        this._onRescan = onRescan;
        this._flows = new Map();
        this.dialog = new Adw.Dialog({ content_width: CONTENT_WIDTH, content_height: CONTENT_HEIGHT });
        this._navigation = new Adw.NavigationView();
        this._navigation.connect('popped', (_view, page) => this._dropFlow(page));
        this.dialog.connect('closed', () => this._cancelAll());
        this.dialog.child = this._navigation;
        const picker = providerPickerPage({
            dir,
            providers,
            error: providersError,
            onPick: provider => this._pickProvider(provider),
        });
        this._navigation.push(picker);
    }

    present(parent) {
        this.dialog.present(parent);
    }

    _pickProvider(provider) {
        if (provider.methods.length === 1) {
            this._startFlow(provider, provider.methods[0]);
            return;
        }
        this._navigation.push(methodPickerPage({ provider, onPick: method => this._startFlow(provider, method) }));
    }

    _startFlow(provider, method) {
        const flow = this._createFlow(provider, method);
        this._flows.set(flow.page, flow);
        this._navigation.push(flow.page);
    }

    _createFlow(provider, method) {
        const onClose = () => this.dialog.close();
        if (method.kind === 'api_key') return new ApiKeyPage({ dir: this._dir, provider, method, onClose });
        if (method.kind === 'auto_detect')
            return new AutoDetectPage({ dir: this._dir, provider, method, onRescan: this._onRescan });
        return new CliLoginPage({ dir: this._dir, provider, method, onClose });
    }

    _dropFlow(page) {
        this._flows.get(page)?.cancel();
        this._flows.delete(page);
    }

    _cancelAll() {
        for (const flow of this._flows.values()) flow.cancel();
        this._flows.clear();
    }
}
