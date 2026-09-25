import GLib from 'gi://GLib';
import { DaemonConnection } from '../daemonConnection.js';
import { remoteMessage } from '../daemonInterface.js';
import { decodeSettings, mergePatch, settingsFrom } from '../settings.js';
import { parseState } from '../state.js';
import { parseProviders, RegistryError } from './registry.js';

const openConnection = options => new DaemonConnection(options);

export class PrefsClient {
    constructor({ onAvailable, onUnavailable, onState, onSettings, onError, connect = openConnection }) {
        this._handlers = { onAvailable, onUnavailable, onState, onSettings, onError };
        this._rawSettings = null;
        this._patchSequence = 0;
        this._deliveryId = 0;
        this.state = null;
        this.settings = null;
        this.providers = null;
        this.providersError = null;
        this._connection = connect({
            signals: { StateChanged: json => this._acceptState(json) },
            onReady: proxy => this._onReady(proxy),
            onVanished: () => this._onVanished(),
            onError: message => this._handlers?.onError(message),
        });
    }

    destroy() {
        this._cancelDelivery();
        this._connection.destroy();
        this._handlers = null;
    }

    updateSettings(patch) {
        if (this._rawSettings) this._acceptOptimistic(mergePatch(this._rawSettings, patch));
        const sequence = ++this._patchSequence;
        this._connection.enqueue(proxy => this._sendPatch(proxy, patch, sequence));
    }

    async _sendPatch(proxy, patch, sequence) {
        try {
            await proxy.UpdateSettingsAsync(JSON.stringify(patch));
        } finally {
            if (sequence === this._patchSequence) this._connection.call(current => this._loadSettings(current));
        }
    }

    setAccountLabel(accountId, label) {
        this._connection.enqueue(proxy => proxy.SetAccountLabelAsync(accountId, label));
    }

    setAccountHidden(accountId, hidden) {
        this._connection.enqueue(proxy => proxy.SetAccountHiddenAsync(accountId, hidden));
    }

    setAccountOrder(ids) {
        this._connection.enqueue(proxy => proxy.SetAccountOrderAsync(ids));
    }

    restoreAccounts(provider) {
        return this._connection.enqueue(proxy => proxy.RestoreAccountsAsync(provider));
    }

    _onReady(proxy) {
        this._connection.call(async current => {
            const [json] = await current.GetStateAsync();
            if (this._connection.proxy !== proxy) return;
            this._acceptState(json);
            await this._loadSettings(current);
            await this._loadProviders(current);
            if (this._connection.proxy === proxy) this._handlers.onAvailable();
        });
    }

    async _loadProviders(proxy) {
        try {
            const [json] = await proxy.ListProvidersAsync();
            if (this._connection.proxy !== proxy) return;
            this.providers = parseProviders(json);
            this.providersError = null;
        } catch (error) {
            if (this._connection.proxy !== proxy) return;
            this.providers = [];
            this.providersError = error instanceof RegistryError ? error.message : remoteMessage(error);
        }
    }

    async _loadSettings(proxy) {
        const sequence = this._patchSequence;
        const [json] = await proxy.GetSettingsAsync();
        if (this._connection.proxy !== proxy || sequence !== this._patchSequence) return;
        this._acceptSettings(decodeSettings(json));
    }

    _acceptSettings(raw) {
        this._cancelDelivery();
        this._storeSettings(raw);
        this._handlers?.onSettings(this.settings);
    }

    _acceptOptimistic(raw) {
        this._storeSettings(raw);
        if (this._deliveryId) return;
        this._deliveryId = GLib.idle_add(GLib.PRIORITY_DEFAULT_IDLE, () => {
            this._deliveryId = 0;
            if (this.settings) this._handlers?.onSettings(this.settings);
            return GLib.SOURCE_REMOVE;
        });
    }

    _storeSettings(raw) {
        this._rawSettings = raw;
        this.settings = settingsFrom(raw);
    }

    _cancelDelivery() {
        if (!this._deliveryId) return;
        GLib.source_remove(this._deliveryId);
        this._deliveryId = 0;
    }

    _acceptState(json) {
        if (!this._handlers) return;
        try {
            this.state = parseState(json);
        } catch (error) {
            this._handlers.onError(error.message);
            return;
        }
        this._handlers.onState(this.state);
    }

    _onVanished() {
        this._cancelDelivery();
        this._rawSettings = null;
        this.state = null;
        this.settings = null;
        this.providers = null;
        this.providersError = null;
        this._handlers?.onUnavailable();
    }
}
