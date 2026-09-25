import QtQuick
import QtTest
import headroom.preview

TestCase {
    id: suite

    readonly property int settleMs: 3000
    readonly property int replyDrainMs: 100
    property Loader plasmoid: null

    function findAll(object, predicate, found) {
        if (predicate(object))
            found.push(object);
        for (const child of object.data ?? [])
            findAll(child, predicate, found);
        return found;
    }

    function everything() {
        return findAll(plasmoid, () => true, []);
    }

    function created(before) {
        const known = new Set(before);
        return everything().filter(item => !known.has(item));
    }

    function host() {
        return plasmoid.item;
    }

    function daemon() {
        return findAll(plasmoid, item => item.patchQueue !== undefined && typeof item.accept === "function", [])[0];
    }

    function dashboard() {
        return findAll(plasmoid, item => typeof item.setFolded === "function", [])[0];
    }

    function named(name) {
        return findAll(plasmoid, item => item.objectName === name, [])[0] ?? null;
    }

    function readings() {
        return findAll(plasmoid, item => item.objectName === "reading", []).map(item => item.text);
    }

    function changedState(step) {
        const raw = JSON.parse(PreviewConfig.shiftedState());
        raw.accounts.forEach(account => (account.windows ?? []).forEach(window => {
                window.used_percent = (window.used_percent + step * 7) % 100;
                window.remaining_percent = 100 - window.used_percent;
            }));
        for (const period of [raw.spend.today, raw.spend.last_30_days]) {
            period.cost_usd_micros += step * 1000;
            period.by_provider[0].cost_usd_micros += step * 1000;
            period.by_provider[0].models[0].cost_usd_micros += step * 1000;
        }
        raw.usage.forEach(usage => usage.daily[usage.daily.length - 1].total_tokens += step);
        return JSON.stringify(raw);
    }

    function open(scenario) {
        PreviewConfig.scenario = scenario ?? "ready";
        PreviewConfig.displayPatch = {
            language: "en"
        };
        PreviewConfig.appliedSettings = null;
        plasmoid = createTemporaryObject(plasmoidComponent, suite) as Loader;
        tryVerify(() => dashboard() !== undefined && dashboard().reveal === 1, settleMs);
        wait(replyDrainMs);
    }

    function cleanup() {
        wait(replyDrainMs);
        PreviewConfig.scenario = "ready";
        PreviewConfig.displayPatch = {};
        plasmoid = null;
    }

    function test_changed_states_update_popup_in_place() {
        open();
        const before = everything();
        const shownBefore = readings();
        for (let step = 1; step <= 3; step++)
            daemon().accept(changedState(step));
        wait(replyDrainMs);
        verify(JSON.stringify(readings()) !== JSON.stringify(shownBefore));
        compare(created(before).length, 0);
    }

    function test_identical_state_keeps_the_view() {
        open();
        const json = PreviewConfig.shiftedState();
        daemon().accept(json);
        const view = daemon().view;
        const before = everything();
        daemon().accept(json);
        verify(daemon().view === view);
        compare(created(before).length, 0);
    }

    function test_identical_settings_keep_the_settings_object() {
        open();
        const client = daemon();
        const settings = client.settings;
        verify(settings !== null);
        client.acceptSettings(PreviewConfig.settingsJson());
        verify(client.settings === settings);
    }

    function test_closed_popup_ignores_states_until_opened() {
        open();
        const snapshot = dashboard().snapshot;
        host().expanded = false;
        daemon().accept(changedState(5));
        verify(dashboard().snapshot === snapshot);
        host().expanded = true;
        verify(dashboard().snapshot === daemon().view.state);
    }

    function test_closed_popup_stops_its_animations() {
        open("refreshing");
        verify(named("refreshGlyph").turning);
        host().expanded = false;
        verify(!named("refreshGlyph").turning);
        host().expanded = true;
        verify(named("refreshGlyph").turning);
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
