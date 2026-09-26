import Adw from 'gi://Adw';
import GLib from 'gi://GLib';
import Gtk from 'gi://Gtk';
import { DesktopClock } from '../desktopClock.js';
import { currentLanguage, resolveLanguage, setLanguage } from '../i18n.js';
import { needsOnboarding } from '../settings.js';
import { UpdateRunner } from '../updateRunner.js';
import { SIDEBAR_CSS } from './accountSidebar.js';
import { AccountsPage } from './accountsPage.js';
import { AdvancedPage } from './advancedPage.js';
import { PrefsClient } from './client.js';
import { GeneralPage } from './generalPage.js';
import { KEYCAPS_CSS } from './keycaps.js';
import { NotificationsPage } from './notificationsPage.js';
import { OnboardingPage } from './onboardingPage.js';
import { PANEL_ART_CSS } from './panelArt.js';
import { DRAG_CSS } from './rowDragger.js';
import { ServicePage } from './servicePage.js';

const DEFAULT_WIDTH = 720;
const DEFAULT_HEIGHT = 820;

export class PrefsController {
    constructor(window, dir) {
        this._window = window;
        this._dir = dir;
        this._shown = [];
        this._onboarding = null;
        this._onboardingOffered = false;
        this._lastVisible = 0;
        this._css = new Gtk.CssProvider();
        this._css.load_from_string([DRAG_CSS, KEYCAPS_CSS, PANEL_ART_CSS, SIDEBAR_CSS].join('\n'));
        Gtk.StyleContext.add_provider_for_display(window.get_display(), this._css, Gtk.STYLE_PROVIDER_PRIORITY_USER);
        window.set_default_size(DEFAULT_WIDTH, DEFAULT_HEIGHT);
        window.search_enabled = false;
        this._clock = new DesktopClock(() => this._refresh());
        this._client = new PrefsClient({
            onAvailable: () => this._refresh(),
            onUnavailable: () => this._showStopped(),
            onState: () => this._refresh(),
            onSettings: () => this._refresh(),
            onError: message => this._toast(message),
        });
        this._updater = new UpdateRunner(() => this._general.syncUpdateRun());
        setLanguage(resolveLanguage('system', GLib.get_language_names()));
        this._buildPages();
        this._showPages([this._service.page]);
    }

    destroy() {
        this._updater.detach();
        this._client.destroy();
        this._destroyPages();
        this._clock.destroy();
        Gtk.StyleContext.remove_provider_for_display(this._window.get_display(), this._css);
    }

    _toast(title) {
        this._window.add_toast(new Adw.Toast({ title, timeout: 5, use_markup: false }));
    }

    _buildPages() {
        const client = this._client;
        this._service = new ServicePage(this._dir);
        this._general = new GeneralPage(client, this._updater, this._dir);
        this._accounts = new AccountsPage({ window: this._window, dir: this._dir, client });
        this._notifications = new NotificationsPage(client, this._dir);
        this._advanced = new AdvancedPage({ client, window: this._window, toast: title => this._toast(title) });
    }

    _destroyPages() {
        this._closeOnboarding();
        this._service.destroy();
        this._notifications.destroy();
        this._advanced.destroy();
    }

    _settingsPages() {
        return [this._general.page, this._accounts.page, this._notifications.page, this._advanced.page];
    }

    _showStopped() {
        this._lastVisible = Math.max(0, this._shown.indexOf(this._window.visible_page));
        this._advanced.disconnected();
        this._service.showStopped();
        this._showPages([this._service.page]);
    }

    _refresh() {
        const { state, settings } = this._client;
        if (!state || !settings) return;
        this._applyLanguage(settings.display.language);
        if (this._shown[0] === this._service.page) this._restorePages();
        this._general.update(settings, state);
        this._accounts.update(state, settings);
        this._notifications.update(settings, state);
        this._advanced.update(settings, state);
        this._syncOnboarding(state, settings);
    }

    _restorePages() {
        this._showPages(this._settingsPages());
        const page = this._shown[this._lastVisible];
        if (page) this._window.visible_page = page;
    }

    _syncOnboarding(state, settings) {
        if (!this._onboarding && !this._onboardingOffered && needsOnboarding(state, settings)) this._openOnboarding();
        this._onboarding?.update(state, settings);
    }

    _openOnboarding() {
        this._onboardingOffered = true;
        this._onboarding = new OnboardingPage({
            dir: this._dir,
            client: this._client,
            clock: this._clock,
            toast: title => this._toast(title),
            actions: {
                addAccount: options => this._accounts.openAddDialog(options),
                signIn: account => this._accounts.signIn(account),
            },
            onFinished: () => this._closeOnboarding(),
        });
        this._window.push_subpage(this._onboarding.page);
    }

    _closeOnboarding() {
        if (!this._onboarding) return;
        this._onboarding.destroy();
        this._onboarding = null;
        this._window.pop_subpage();
    }

    _applyLanguage(setting) {
        const language = resolveLanguage(setting, GLib.get_language_names());
        if (language === currentLanguage()) return;
        setLanguage(language);
        const visible = this._shown.indexOf(this._window.visible_page);
        const reopen = this._onboarding !== null;
        this._showPages([]);
        this._destroyPages();
        this._buildPages();
        this._showPages(this._settingsPages());
        const page = this._shown[visible];
        if (page) this._window.visible_page = page;
        if (reopen) this._onboardingOffered = false;
    }

    _showPages(pages) {
        if (pages.length === this._shown.length && pages.every((page, index) => page === this._shown[index])) return;
        for (const page of this._shown) this._window.remove(page);
        for (const page of pages) this._window.add(page);
        this._shown = pages;
    }
}
