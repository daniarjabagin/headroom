import Adw from 'gi://Adw';
import GLib from 'gi://GLib';
import Gtk from 'gi://Gtk';
import { currentLanguage, resolveLanguage, setLanguage } from '../i18n.js';
import { AccountsPage } from './accountsPage.js';
import { PrefsClient } from './client.js';
import { GeneralPage } from './generalPage.js';
import { NotificationsPage } from './notificationsPage.js';
import { DRAG_CSS } from './rowDragger.js';
import { ServicePage } from './servicePage.js';

const DEFAULT_WIDTH = 680;
const DEFAULT_HEIGHT = 820;

export class PrefsController {
    constructor(window, dir) {
        this._window = window;
        this._dir = dir;
        this._shown = [];
        this._css = new Gtk.CssProvider();
        this._css.load_from_string(DRAG_CSS);
        Gtk.StyleContext.add_provider_for_display(window.get_display(), this._css, Gtk.STYLE_PROVIDER_PRIORITY_USER);
        window.set_default_size(DEFAULT_WIDTH, DEFAULT_HEIGHT);
        window.search_enabled = false;
        this._client = new PrefsClient({
            onAvailable: () => this._refresh(),
            onUnavailable: () => this._showStopped(),
            onState: () => this._refresh(),
            onSettings: () => this._refresh(),
            onError: message => window.add_toast(new Adw.Toast({ title: message, timeout: 5 })),
        });
        setLanguage(resolveLanguage('system', GLib.get_language_names()));
        this._buildPages();
        this._showPages([this._service.page]);
    }

    destroy() {
        this._client.destroy();
        this._service.destroy();
        Gtk.StyleContext.remove_provider_for_display(this._window.get_display(), this._css);
    }

    _buildPages() {
        this._service = new ServicePage(this._dir);
        this._general = new GeneralPage(this._client);
        this._accounts = new AccountsPage({ window: this._window, dir: this._dir, client: this._client });
        this._notifications = new NotificationsPage(this._client);
    }

    _showStopped() {
        this._service.showStopped();
        this._showPages([this._service.page]);
    }

    _refresh() {
        const { state, settings } = this._client;
        if (!state || !settings) return;
        this._applyLanguage(settings.display.language);
        this._showPages([this._general.page, this._accounts.page, this._notifications.page]);
        this._general.update(settings, state);
        this._accounts.update(state, settings);
        this._notifications.update(settings);
    }

    _applyLanguage(setting) {
        const language = resolveLanguage(setting, GLib.get_language_names());
        if (language === currentLanguage()) return;
        setLanguage(language);
        const visible = this._shown.indexOf(this._window.visible_page);
        this._showPages([]);
        this._service.destroy();
        this._buildPages();
        this._showPages([this._general.page, this._accounts.page, this._notifications.page]);
        const page = this._shown[visible];
        if (page) this._window.visible_page = page;
    }

    _showPages(pages) {
        if (pages.length === this._shown.length && pages.every((page, index) => page === this._shown[index])) return;
        for (const page of this._shown) this._window.remove(page);
        for (const page of pages) this._window.add(page);
        this._shown = pages;
    }
}
