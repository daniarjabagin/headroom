import QtQuick
import QtTest
import org.kde.plasma.workspace.dbus as DBus
import headroom.preview

TestCase {
    id: suite

    readonly property int settleMs: 3000
    readonly property int replyDrainMs: 100
    property Item plasmoid: null

    function findAll(object, predicate, found) {
        if (predicate(object))
            found.push(object);
        for (const child of object.data ?? [])
            findAll(child, predicate, found);
        return found;
    }

    function named(name) {
        return findAll(plasmoid, item => item.objectName === name, [])[0] ?? null;
    }

    function sections() {
        return findAll(plasmoid, item => item.account !== undefined && item.notices !== undefined && item.windows !== undefined, []);
    }

    function section(provider) {
        return sections().find(item => item.account.provider === provider && item.windows.length > 0);
    }

    function dashboard() {
        return findAll(plasmoid, item => typeof item.setFolded === "function", [])[0];
    }

    function spendCard() {
        return findAll(plasmoid, item => item.body !== undefined && item.periodSelected !== undefined, [])[0];
    }

    function calls(member) {
        return DBus.SessionBus.messages.filter(message => message.member === member);
    }

    function patches() {
        return calls("UpdateSettings").map(message => JSON.parse(message.arguments[0]));
    }

    function rawSample() {
        const request = new XMLHttpRequest();
        request.open("GET", Qt.resolvedUrl("../dev/sample-state.json"), false);
        request.send();
        return JSON.parse(request.responseText);
    }

    function open(display, statePatch) {
        PreviewConfig.scenario = "ready";
        PreviewConfig.displayPatch = Object.assign({
            language: "en"
        }, display ?? {});
        PreviewConfig.statePatch = statePatch ?? {};
        PreviewConfig.appliedSettings = null;
        plasmoid = createTemporaryObject(plasmoidComponent, suite) as Item;
        tryVerify(() => sections().length > 0, settleMs);
        DBus.SessionBus.messages = [];
    }

    function cleanup() {
        wait(replyDrainMs);
        PreviewConfig.displayPatch = {};
        PreviewConfig.statePatch = {};
        PreviewConfig.appliedSettings = null;
        plasmoid = null;
    }

    function test_period_choice_is_saved_and_shown() {
        open();
        const card = spendCard();
        compare(card.period, "30d");
        compare(named("periodControl").options.map(option => option.value), ["today", "yesterday", "7d", "30d"]);
        card.periodSelected("7d");
        compare(card.period, "7d");
        compare(card.current, card.spend.last7Days);
        tryVerify(() => patches().length === 1, settleMs);
        compare(patches()[0], {
            display: {
                spend_period: "7d"
            }
        });
    }

    function test_unit_choice_changes_ring_and_is_saved() {
        open();
        const card = spendCard();
        card.unitSelected("tokens");
        compare(card.unit, "tokens");
        const ring = findAll(card, item => item.objectName === "ringValue", [])[0];
        verify(ring.text.endsWith("M") || ring.text.endsWith("K"));
        tryVerify(() => patches().length === 1, settleMs);
        compare(patches()[0].display.spend_unit, "tokens");
        compare(named("unitTitle").Accessible.name, "Total Tokens");
    }

    function test_breakdown_switches_to_projects() {
        open();
        const list = named("spendBreakdown");
        verify(list.visible);
        compare(list.mode, "models");
        spendCard().breakdownSelected("projects");
        compare(list.mode, "projects");
        verify(list.model.rows.every(row => row.folder));
        tryVerify(() => patches().length === 1, settleMs);
        compare(patches()[0].display.spend_breakdown, "projects");
    }

    function test_value_click_toggles_value_mode() {
        open();
        const reading = findAll(section("codex"), item => item.objectName === "reading", [])[0];
        verify(reading.text.endsWith("left"));
        tryVerify(() => dashboard().reveal === 1, settleMs);
        mouseClick(reading);
        tryVerify(() => reading.text.endsWith("used"), settleMs);
        compare(patches()[0].display.value_mode, "used");
    }

    function test_menu_hides_refreshes_and_stars_the_account() {
        open();
        const codex = section("codex");
        codex.hideRequested();
        codex.menuRefreshRequested();
        codex.starToggled();
        tryVerify(() => calls("SetAccountHidden").length === 1, settleMs);
        compare(calls("SetAccountHidden")[0].arguments, [codex.account.id, true]);
        tryVerify(() => calls("Refresh").length === 1, settleMs);
        compare(calls("Refresh")[0].arguments, [codex.account.id]);
        tryVerify(() => patches().length === 1, settleMs);
        compare(patches()[0].display.starred_accounts, [codex.account.id]);
    }

    function test_header_offers_registry_links() {
        open();
        const header = findAll(section("claude"), item => item.headerLinks !== undefined, [])[0];
        tryVerify(() => header.headerLinks.length === 2, settleMs);
        compare(header.headerLinks.map(entry => entry.kind), ["status", "dashboard"]);
        compare(header.headerLinks[0].tip, "Status page · status.claude.com");
        compare(findAll(header, item => item.objectName === "quickLinks", [])[0].opacity, 0);
    }

    function test_incident_shows_notice_in_the_card() {
        open();
        const claude = section("claude");
        verify(claude.incident !== null);
        compare(claude.incident.title, "Degraded performance · Elevated errors on Claude Code");
        tryVerify(() => findAll(claude, item => item.objectName === "statusNotice", []).length === 1, settleMs);
        verify(findAll(claude, item => item.objectName === "incidentIcon", [])[0].visible);
        compare(section("codex").incident, null);
    }

    function test_collapsed_accounts_fold_into_one_row() {
        const raw = rawSample();
        raw.accounts.filter(account => account.provider === "codex").forEach(account => account.collapsed = true);
        open({}, {
            accounts: raw.accounts
        });
        const row = named("collapsedRow");
        verify(row.visible);
        verify(row.Accessible.name.startsWith("3 more · Codex"));
        const shownBefore = sections().length;
        dashboard().reducedMotion = true;
        row.clicked();
        tryVerify(() => sections().length === shownBefore + 3, settleMs);
        verify(!row.visible);
        verify(named("foldDivider").visible);
        named("showLess").clicked();
        tryVerify(() => sections().length === shownBefore, settleMs);
    }

    function test_compact_density_uses_one_line_rows() {
        open({
            density: "compact"
        });
        const reading = findAll(section("codex"), item => item.objectName === "compactReading", [])[0];
        verify(reading.visible);
        verify(!findAll(section("codex"), item => item.objectName === "reading", [])[0].visible);
    }

    function test_footer_shows_live_polling() {
        const raw = rawSample();
        raw.accounts.forEach(account => account.refresh = {
                mode: account.provider === "codex" ? "live" : "idle",
                interval_secs: 60,
                next_at: null,
                reason: account.provider === "codex" ? "activity" : "schedule"
            });
        open({}, {
            accounts: raw.accounts
        });
        tryVerify(() => named("liveDot").visible, settleMs);
        compare(named("footerStatus").Accessible.name, "Live — every 1m · Codex");
    }

    function test_copy_as_text_confirms_with_toast() {
        open();
        section("codex").copyRequested();
        const toast = named("toast");
        tryVerify(() => toast.visible, settleMs);
        compare(toast.text, "Copied as text");
    }

    function exporter() {
        return findAll(plasmoid, item => item.picturesUrl !== undefined, [])[0];
    }

    function pendingShare(folder) {
        return {
            shared: null,
            members: [],
            folder,
            path: `${folder}/headroom-codex.png`
        };
    }

    function test_share_saves_and_offers_the_folder() {
        open();
        const target = exporter();
        const saved = [];
        target.pending = pendingShare("/data/Pictures/Headroom");
        target.save({
            saveToFile: path => saved.push(path) > 0
        });
        compare(saved, ["/data/Pictures/Headroom/headroom-codex.png"]);
        const toast = named("toast");
        tryVerify(() => toast.visible, settleMs);
        compare(toast.text, "Saved to Pictures/Headroom");
        compare(toast.actionUrl, "file:///data/Pictures/Headroom");
        verify(!toast.failed);
        verify(!target.busy);
    }

    function test_share_creates_the_folder_then_reports_failure() {
        open();
        const target = exporter();
        target.pending = pendingShare("/proc/headroom-test/Headroom");
        target.save({
            saveToFile: () => false
        });
        const runner = findAll(target, item => item.item?.engine === "executable", [])[0].item;
        compare(runner.connectedSources, ["mkdir -p '/proc/headroom-test/Headroom'"]);
        runner.newData(runner.connectedSources[0], {
            "exit code": 1,
            stdout: ""
        });
        const toast = named("toast");
        tryVerify(() => toast.visible && toast.failed, settleMs);
        compare(toast.text, "Could not save the image");
        verify(!target.busy);
    }

    function test_share_without_limits_fails_at_once() {
        open();
        exporter().share({
            provider: "codex",
            hero: null
        }, []);
        tryVerify(() => named("toast").failed, settleMs);
    }

    function test_onboarding_banner_dismisses_through_settings() {
        open();
        const banner = named("onboardingBanner");
        verify(banner !== null);
        banner.dismissed();
        tryVerify(() => patches().some(patch => patch.onboarding?.completed === true), settleMs);
    }

    width: 800
    height: 1600
    visible: true
    when: windowShown

    Component {
        id: plasmoidComponent

        Loader {
            source: "../package/contents/ui/main.qml"
        }
    }
}
