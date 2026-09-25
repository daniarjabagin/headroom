import QtQuick
import QtTest
import org.kde.plasma.workspace.dbus as DBus
import headroom.preview
import "../package/contents/ui"
import "../package/contents/ui/logic/Settings.js" as Settings
import "../package/contents/ui/logic/State.js" as State

TestCase {
    id: suite

    readonly property int settleMs: 3000
    readonly property int replyDrainMs: 100
    property Loader root: null

    function findAll(object, predicate, found) {
        if (predicate(object))
            found.push(object);
        for (const child of object.data ?? [])
            findAll(child, predicate, found);
        return found;
    }

    function named(name) {
        return findAll(root, item => item.objectName === name, [])[0] ?? null;
    }

    function allNamed(name) {
        return findAll(root, item => item.objectName === name, []);
    }

    function page(): ConfigScaffold {
        return root.item as ConfigScaffold;
    }

    function applied() {
        return PreviewConfig.currentSettings();
    }

    function load(pageName, statePatch) {
        PreviewConfig.scenario = "ready";
        PreviewConfig.displayPatch = {
            language: "en"
        };
        PreviewConfig.appliedSettings = null;
        PreviewConfig.statePatch = Object.assign({
            app_version: "0.6.0"
        }, statePatch ?? {});
        root = createTemporaryObject(pageComponent, suite, {
            source: `../package/contents/ui/Config${pageName}.qml`
        }) as Loader;
        tryVerify(() => page() !== null && page().ready, settleMs);
        wait(replyDrainMs);
    }

    function cleanup() {
        wait(replyDrainMs);
        PreviewConfig.appliedSettings = null;
        PreviewConfig.statePatch = {};
        PreviewConfig.displayPatch = {};
        root = null;
    }

    function test_general_shows_new_rows_only_for_new_daemons() {
        load("General");
        verify(named("densityRow").visible);
        verify(named("adaptiveRefresh").visible);
        verify(named("statusPages").visible);
        verify(named("shortcutRecorder").visible);
        cleanup();
        load("General", {
            panel_items: undefined
        });
        verify(!named("densityRow").visible);
        verify(!named("adaptiveRefresh").visible);
        verify(!named("statusPages").visible);
        verify(!named("shortcutRecorder").visible);
    }

    function test_star_writes_starred_accounts() {
        load("General");
        const stars = allNamed("starButton");
        verify(stars.length > 0);
        mouseClick(stars[0]);
        tryVerify(() => (applied().display.starred_accounts ?? []).length === 1, settleMs);
        const first = State.visibleAccounts(page().snapshot)[0].id;
        compare(applied().display.starred_accounts, [first]);
    }

    function test_panel_limits_picker_writes_limits() {
        load("General");
        page().setDisplay("panelMode", "several");
        tryVerify(() => named("panelLimitsRow").visible, settleMs);
        verify(allNamed("panelLimitChoice").length > 3);
        for (let index = 0; index < 3; index++) {
            mouseClick(allNamed("panelLimitChoice")[index]);
            tryVerify(() => (applied().display.panel_limits ?? []).length === index + 1, settleMs);
        }
        tryVerify(() => (applied().display.panel_limits ?? []).length === 3, settleMs);
        tryVerify(() => !allNamed("panelLimitChoice")[3].enabled, settleMs);
        compare(named("panelLimitsRow").subtitle, "3 of 3 chosen · shown in this order");
    }

    function test_shortcut_capture_writes_a_gtk_accelerator() {
        load("General");
        const recorder = named("shortcutRecorder");
        recorder.keySequence = "Meta+Shift+U";
        recorder.captureFinished();
        tryVerify(() => applied().shortcuts.open === "<Shift><Super>u", settleMs);
        compare(recorder.keySequence, "Shift+Meta+U");
        recorder.keySequence = "Ctrl+K, Ctrl+U";
        recorder.captureFinished();
        compare(recorder.keySequence, "Shift+Meta+U");
        compare(applied().shortcuts.open, "<Shift><Super>u");
    }

    function test_notifications_threshold_and_quiet_hours() {
        load("Notifications");
        compare(named("milestone-almostOut").subtitle, "A limit drops under 10% left");
        page().updateSettings({
            notifications: {
                threshold_percent: 20
            }
        });
        tryVerify(() => named("milestone-almostOut").subtitle === "A limit drops under 20% left", settleMs);
        mouseClick(named("perProviderToggle"));
        tryVerify(() => named("threshold-claude") !== null, settleMs);
        named("threshold-claude").picked(0);
        tryVerify(() => applied().notifications.provider_thresholds.claude === 0, settleMs);
        tryVerify(() => named("perProviderRow").subtitle === "1 provider differs from the default", settleMs);
        const from = named("quietFrom");
        from.text = "23:30";
        from.editingFinished();
        tryVerify(() => applied().notifications.quiet_hours.from === "23:30", settleMs);
        from.text = "99";
        from.editingFinished();
        compare(from.text, "23:30");
    }

    function test_advanced_diagnostics_and_reset() {
        load("Advanced");
        tryVerify(() => named("logFileRow").subtitle === "~/.local/state/headroom/headroom.log", settleMs);
        DBus.SessionBus.messages = [];
        mouseClick(named("copyDiagnostics"));
        tryVerify(() => named("copyDiagnostics").text === "Copied", settleMs);
        page().updateSettings(Settings.displayPatch({
            density: "compact"
        }));
        tryVerify(() => applied().display.density === "compact", settleMs);
        const dialog = findAll(root, item => item instanceof ResetDialog, [])[0];
        dialog.resetConfirmed();
        tryVerify(() => DBus.SessionBus.messages.some(message => message.member === "ResetSettings"), settleMs);
        tryVerify(() => applied().display.density === "normal", settleMs);
    }

    function test_onboarding_banner() {
        const banner = createTemporaryObject(bannerComponent, suite) as OnboardingBanner;
        verify(banner.visible);
        compare(findAll(banner, item => item.objectName === "onboardingTitle", [])[0].text, "Welcome to Headroom — we found Claude and Codex");
        banner.settings = Settings.fromRaw({
            onboarding: {
                completed: true
            }
        });
        verify(!banner.visible);
        banner.capable = false;
        banner.settings = Settings.fromRaw({});
        verify(!banner.visible);
    }

    width: 800
    height: 1600
    visible: true
    when: windowShown

    Component {
        id: pageComponent

        Loader {
            width: 700
            height: 1600
        }
    }

    Component {
        id: bannerComponent

        OnboardingBanner {
            width: 320
            lang: "en"
            capable: true
            settings: Settings.fromRaw({})
            snapshot: ({
                    accounts: [
                        {
                            id: "claude:0a",
                            providerName: "Claude",
                            hidden: false
                        },
                        {
                            id: "codex:1b",
                            providerName: "Codex",
                            hidden: false
                        }
                    ]
                })
        }
    }
}
