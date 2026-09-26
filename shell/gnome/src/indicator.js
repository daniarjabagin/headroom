import Clutter from 'gi://Clutter';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import GObject from 'gi://GObject';
import St from 'gi://St';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import { supports06 } from './compat.js';
import { DaemonClient } from './dbus.js';
import { Glass } from './glass.js';
import { _, currentLanguage, resolveLanguage, setLanguage } from './i18n.js';
import { Motion } from './motion.js';
import { isLightPanel, panelLayout } from './panelContent.js';
import { PanelDrag } from './panelDrag.js';
import { PanelItemsView } from './panelItemsView.js';
import { PanelPlacement } from './panelPlacement.js';
import { LoginLauncher } from './popup/loginLauncher.js';
import { PopupView } from './popup/popup.js';
import { Privacy } from './privacy.js';
import { startService } from './service.js';
import { displayPatch, panelPositionPatch, toggledResetFormat, toggledValueMode } from './settings.js';
import { GlobalShortcut } from './shortcut.js';
import { parseState, StateError } from './state.js';
import { Ticker } from './ticker.js';
import { UpdateRunner } from './updateRunner.js';

const WORK_AREA_GAP = 16;
const THEME_CLASSES = ['headroom-theme-light', 'headroom-theme-dark'];
const REDUCED_MOTION_CLASS = 'headroom-reduced-motion';

const ONBOARDING_MODULE = 'onboarding.js';

async function loadOnboarding(dir) {
    if (!dir.get_child('src').get_child(ONBOARDING_MODULE).query_exists(null)) return null;
    const module = await import(`./${ONBOARDING_MODULE}`);
    return typeof module.maybeShowOnboarding === 'function' ? module.maybeShowOnboarding : null;
}

function themeClass(display) {
    return display.theme === 'system' ? '' : `headroom-theme-${display.theme}`;
}

export const Indicator = GObject.registerClass(
    class HeadroomIndicator extends PanelMenu.Button {
        _init(extension) {
            super._init(0.5, 'Headroom', false);
            this._clickGesture?.set_enabled(false);
            this._extension = extension;
            this._view = { kind: 'loading', state: null };
            this._theme = null;
            this._motion = new Motion();
            this._cancellable = new Gio.Cancellable();
            this._onboarding = { requested: false, destroyed: false };
            setLanguage(resolveLanguage('system', GLib.get_language_names()));
            this._ticker = new Ticker({
                onTick: () => this._popup.tick(),
                wantsSeconds: () => this._popup.needsSecondTicks(),
            });
            this._panelView = new PanelItemsView({ dir: extension.dir, motion: this._motion });
            this.add_child(this._panelView.actor);
            this._styleChangedId = this.connect('style-changed', () => this._syncPanelTheme());
            this._privacy = new Privacy();
            this._privacy.connect('changed', () => this._render());
            this._shortcut = new GlobalShortcut(() => this.menu.toggle());
            this._login = new LoginLauncher(message => Main.notifyError(_("Couldn't start sign-in"), message));
            this._popup = this._createPopup();
            this._updater = new UpdateRunner(run => this._popup.setUpdateRun(run));
            this.menu.actor.add_style_class_name('headroom-menu');
            this._glass = new Glass(this.menu);
            this.menu.box.add_child(this._popup.actor);
            this._menuToggledId = this.menu.connect('open-state-changed', (_menu, open) => this._onMenuToggled(open));
            this._client = new DaemonClient({
                onAvailable: () => this._setView({ kind: 'loading', state: null }),
                onUnavailable: () => this._setView({ kind: 'unavailable', state: null }),
                onState: json => this._onState(json),
                onSettings: settings => this._onSettings(settings),
                onError: message => this._onError(message),
                onOpenRequested: () => this.menu.open(),
            });
            this._render();
        }

        addToPanel(panel, role) {
            this._placement = new PanelPlacement(panel, role, this);
            this._drag = new PanelDrag({
                button: this,
                placement: this._placement,
                onDrop: position => this._client.updateSettings(panelPositionPatch(position)),
                onClick: () => this.menu.toggle(),
            });
            this._placement.attach();
            this._renderPlacement();
        }

        vfunc_event(event) {
            return this._drag?.handleEvent(event) ? Clutter.EVENT_STOP : Clutter.EVENT_PROPAGATE;
        }

        _createPopup() {
            return new PopupView({
                dir: this._extension.dir,
                versionText: `Headroom ${this._extension.metadata['version-name'] ?? ''}`.trim(),
                motion: this._motion,
                actions: {
                    refresh: accountId => this._client.refresh(accountId),
                    refreshNow: () => this._client.refreshNow(),
                    setOrder: ids => this._client.setAccountOrder(ids),
                    setHidden: (accountId, hidden) => this._client.setAccountHidden(accountId, hidden),
                    listProviders: () => this._client.listProviders(),
                    updateDisplay: (changes, patch) => this._updateDisplay(changes, patch),
                    toggleValueMode: () => this._patchDisplay(toggledValueMode),
                    toggleResetFormat: () => this._patchDisplay(toggledResetFormat),
                    copy: text => St.Clipboard.get_default().set_text(St.ClipboardType.CLIPBOARD, text),
                    openPreferences: () => this._openPreferences(),
                    openUrl: url => this._openUrl(url),
                    signIn: accountId => this._signIn(accountId),
                    installUpdate: () => this._updater.start(),
                    startService: () => this._startService(),
                    showNumbersAnyway: () => this._privacy.showAnyway(),
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

        _onSettings(settings) {
            this._applyMotion(settings.reducedMotion);
            this._shortcut.set(settings.shortcuts.open);
            this._maybeOnboard(settings);
        }

        _applyMotion(reduced) {
            this._motion.reduced = reduced;
            if (reduced) this.menu.actor.add_style_class_name(REDUCED_MOTION_CLASS);
            else this.menu.actor.remove_style_class_name(REDUCED_MOTION_CLASS);
            this._panelView.syncMotion();
        }

        async _maybeOnboard(settings) {
            if (this._onboarding.requested || this._view.kind !== 'ready') return;
            this._onboarding.requested = true;
            const state = this._view.state;
            try {
                const show = await loadOnboarding(this._extension.dir);
                if (!this._onboarding.destroyed) show?.(this._extension, state, settings);
            } catch (error) {
                console.error(`Headroom: onboarding failed: ${error.message}`);
            }
        }

        _patchDisplay(patchFor) {
            const state = this._view.state;
            if (this._view.kind !== 'ready' || !state) return;
            const changes = patchFor(state.display);
            this._updateDisplay(changes, displayPatch(changes));
        }

        _updateDisplay(changes, patch) {
            const state = this._view.state;
            if (this._view.kind !== 'ready' || !state) return;
            state.display = { ...state.display, ...changes };
            this._render();
            this._client.updateSettings(patch);
        }

        _render() {
            this._applyLanguage();
            this._applyTheme();
            const state = this._view.state;
            if (state) this._privacy.setEnabled(state.display.hideOnScreenShare);
            const masked = this._privacy.isMasked();
            this._glass.setEnabled(state?.display.translucent ?? false);
            this._panelView.render(panelLayout(state, { masked, legacy: !supports06(state) }));
            this._renderPlacement();
            this._popup.render(this._view, { masked });
            this._ticker.sync();
        }

        _renderPlacement() {
            const state = this._view.state;
            const current = state !== null && supports06(state);
            if (this._drag) this._drag.enabled = current;
            if (current) this._placement?.place(state.display.panelPosition);
        }

        _syncPanelTheme() {
            this._panelView.setLightPanel(isLightPanel(this.get_theme_node().get_foreground_color()));
        }

        _applyLanguage() {
            const setting = this._view.state?.display.language ?? 'system';
            const language = resolveLanguage(setting, GLib.get_language_names());
            if (language === currentLanguage()) return;
            setLanguage(language);
            this._popup.relabel();
        }

        _applyTheme() {
            const theme = this._view.state ? themeClass(this._view.state.display) : '';
            if (theme === this._theme) return;
            this._theme = theme;
            for (const name of THEME_CLASSES) this.menu.actor.remove_style_class_name(name);
            if (theme) this.menu.actor.add_style_class_name(theme);
            this._popup.setTheme(theme);
        }

        _onMenuToggled(open) {
            if (open) {
                this._client.refresh('');
                this._fitToWorkArea();
                this._popup.onOpen();
                this._ticker.start();
            } else {
                this._ticker.stop();
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

        _openPreferences() {
            this.menu.close();
            this._extension.openPreferences();
        }

        _openUrl(url) {
            this.menu.close();
            try {
                Gio.AppInfo.launch_default_for_uri(url, global.create_app_launch_context(0, -1));
            } catch (error) {
                if (!(error instanceof GLib.Error)) throw error;
                Main.notifyError(_("Couldn't open the release page"), error.message);
            }
        }

        _signIn(accountId) {
            this.menu.close();
            this._login.launch(accountId);
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
            this._onboarding.destroyed = true;
            this._drag?.destroy();
            this.disconnect(this._styleChangedId);
            this._shortcut.destroy();
            this._login.destroy();
            this._privacy.destroy();
            this._panelView.destroy();
            this.menu.disconnect(this._menuToggledId);
            this._ticker.stop();
            this._glass.destroy();
            this._cancellable.cancel();
            this._updater.detach();
            this._client.destroy();
            this._popup.destroy();
            super._onDestroy();
        }
    }
);
