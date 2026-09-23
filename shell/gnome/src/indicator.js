import Clutter from 'gi://Clutter';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import GObject from 'gi://GObject';
import St from 'gi://St';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import { DaemonClient } from './dbus.js';
import { panelPercent, shortWindowLabel } from './format.js';
import { PanelRing } from './panelRing.js';
import { PopupView } from './popup/popup.js';
import { startService } from './service.js';
import { toggledResetFormat, toggledValueMode } from './settings.js';
import { parseState, StateError } from './state.js';
import { fileIcon, label, providerIcon, row } from './widgets.js';

const TICK_SECONDS = 30;
const STALE_OPACITY = 140;
const WORK_AREA_GAP = 16;
const WINDOW_LABEL_OPACITY = 170;
const THEME_CLASSES = ['headroom-theme-light', 'headroom-theme-dark'];

function headlineAccount(state) {
    return state.accounts.find(candidate => candidate.id === state.headline.accountId) ?? null;
}

function isStale(state) {
    if (state.offline) return true;
    return headlineAccount(state)?.status === 'stale';
}

function panelValue(headline, display) {
    if (display.valueMode === 'used' && headline.usedPercent !== null) return headline.usedPercent;
    return headline.remainingPercent;
}

function themeClass(display) {
    return display.theme === 'system' ? '' : `headroom-theme-${display.theme}`;
}

export const Indicator = GObject.registerClass(
    class HeadroomIndicator extends PanelMenu.Button {
        _init(extension) {
            super._init(0.5, 'Headroom', false);
            this._extension = extension;
            this._panelProvider = null;
            this._view = { kind: 'loading', state: null };
            this._tickId = 0;
            this._cancellable = new Gio.Cancellable();
            this._buildPanel();
            this._popup = this._createPopup();
            this.menu.actor.add_style_class_name('headroom-menu');
            this.menu.box.add_child(this._popup.actor);
            this._menuToggledId = this.menu.connect('open-state-changed', (_menu, open) => this._onMenuToggled(open));
            this._client = new DaemonClient({
                onAvailable: () => this._setView({ kind: 'loading', state: null }),
                onUnavailable: () => this._setView({ kind: 'unavailable', state: null }),
                onState: json => this._onState(json),
                onError: message => this._onError(message),
                onOpenRequested: () => this.menu.open(),
            });
            this._render();
        }

        _buildPanel() {
            this._panelBox = row({ style_class: 'headroom-panel-box' });
            this._mark = fileIcon(
                this._extension.dir,
                'headroom-symbolic.svg',
                'system-status-icon headroom-panel-mark'
            );
            this._ring = new PanelRing();
            this._ring.y_align = Clutter.ActorAlign.CENTER;
            this._providerSlot = new St.Bin({
                style_class: 'headroom-panel-provider',
                y_align: Clutter.ActorAlign.CENTER,
            });
            this._windowLabel = label('', 'headroom-panel-window');
            this._windowLabel.opacity = WINDOW_LABEL_OPACITY;
            this._percent = label('', 'headroom-panel-label');
            for (const actor of [this._mark, this._ring, this._providerSlot, this._windowLabel, this._percent])
                this._panelBox.add_child(actor);
            this.add_child(this._panelBox);
        }

        _createPopup() {
            return new PopupView({
                dir: this._extension.dir,
                versionText: `Headroom ${this._extension.metadata['version-name'] ?? ''}`.trim(),
                actions: {
                    refresh: accountId => this._client.refresh(accountId),
                    setHidden: (accountId, hidden) => this._client.setAccountHidden(accountId, hidden),
                    setOrder: ids => this._client.setAccountOrder(ids),
                    toggleValueMode: () => this._patchDisplay(toggledValueMode),
                    toggleResetFormat: () => this._patchDisplay(toggledResetFormat),
                    copy: text => St.Clipboard.get_default().set_text(St.ClipboardType.CLIPBOARD, text),
                    openPreferences: () => this._openPreferences(),
                    startService: () => this._startService(),
                },
            });
        }

        _onState(json) {
            try {
                this._setView({ kind: 'ready', state: parseState(json) });
            } catch (error) {
                if (!(error instanceof StateError)) throw error;
                this._setView({ kind: 'error', state: null, error: error.message });
            }
        }

        _onError(message) {
            if (this._view.kind === 'ready') return;
            this._setView({ kind: 'error', state: null, error: message });
        }

        _setView(view) {
            this._view = view;
            this._render();
        }

        _patchDisplay(patchFor) {
            const state = this._view.state;
            if (this._view.kind !== 'ready' || !state) return;
            const patch = patchFor(state.display);
            state.display = { ...state.display, ...patch };
            this._render();
            this._client.updateDisplay(() => patch);
        }

        _render() {
            this._applyTheme();
            this._renderPanel();
            this._popup.render(this._view);
        }

        _applyTheme() {
            const theme = this._view.state ? themeClass(this._view.state.display) : '';
            for (const name of THEME_CLASSES) this.menu.actor.remove_style_class_name(name);
            if (theme) this.menu.actor.add_style_class_name(theme);
            this._popup.setTheme(theme);
        }

        _renderPanel() {
            const state = this._view.state;
            const headline = state?.headline ?? null;
            this._mark.visible = headline === null;
            this._percent.visible = headline !== null;
            const windowMode = headline !== null && state.display.panelLabel === 'window';
            this._ring.visible = headline !== null && !windowMode;
            this._providerSlot.visible = windowMode;
            this._windowLabel.visible = windowMode;
            this._panelBox.opacity = headline && isStale(state) ? STALE_OPACITY : 255;
            if (headline === null) return;
            const value = panelValue(headline, state.display);
            this._ring.update(value / 100, headline.tone);
            this._percent.text = panelPercent(value);
            if (windowMode) this._renderWindowLabel(state, headline);
        }

        _renderWindowLabel(state, headline) {
            const provider = headline.provider ?? headlineAccount(state)?.provider ?? 'unknown';
            if (provider !== this._panelProvider) {
                this._providerSlot.child?.destroy();
                this._providerSlot.set_child(
                    providerIcon(this._extension.dir, provider, 'headroom-panel-provider-icon')
                );
                this._panelProvider = provider;
            }
            this._windowLabel.text = shortWindowLabel(headline.windowId, headline.windowLabel);
        }

        _onMenuToggled(open) {
            if (open) {
                this._client.refresh('');
                this._fitToWorkArea();
                this._popup.onOpen();
                this._startTicking();
            } else {
                this._stopTicking();
                this._popup.onClose();
            }
        }

        _fitToWorkArea() {
            const monitor = Main.layoutManager.findIndexForActor(this);
            const workArea = Main.layoutManager.getWorkAreaForMonitor(monitor);
            const scale = St.ThemeContext.get_for_stage(global.stage).scale_factor;
            const margins = this.menu.actor.margin_top + this.menu.actor.margin_bottom;
            this._popup.setMaxHeight(Math.floor((workArea.height - margins) / scale) - WORK_AREA_GAP);
        }

        _startTicking() {
            this._stopTicking();
            this._tickId = GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, TICK_SECONDS, () => {
                this._popup.tick();
                return GLib.SOURCE_CONTINUE;
            });
        }

        _stopTicking() {
            if (this._tickId === 0) return;
            GLib.source_remove(this._tickId);
            this._tickId = 0;
        }

        _openPreferences() {
            this.menu.close();
            this._extension.openPreferences();
        }

        async _startService() {
            this._setView({ kind: 'unavailable', state: null, starting: true });
            try {
                await startService(this._cancellable);
            } catch (error) {
                if (this._cancellable.is_cancelled()) return;
                if (this._view.kind === 'unavailable')
                    this._setView({ kind: 'unavailable', state: null, startError: error.message });
                return;
            }
            if (this._view.starting) this._setView({ kind: 'unavailable', state: null });
        }

        _onDestroy() {
            this.menu.disconnect(this._menuToggledId);
            this._stopTicking();
            this._cancellable.cancel();
            this._client.destroy();
            this._popup.destroy();
            super._onDestroy();
        }
    }
);
