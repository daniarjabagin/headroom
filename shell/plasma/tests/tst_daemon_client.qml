import QtQuick
import QtTest
import org.kde.plasma.workspace.dbus as DBus
import headroom.preview
import "../package/contents/ui"

TestCase {
    id: suite

    readonly property int settleMs: 2000
    property DaemonClient client: null
    readonly property int sentPatches: DBus.SessionBus.messages.filter(message => message.member === "UpdateSettings").length

    function members() {
        return DBus.SessionBus.messages.map(message => message.member);
    }

    function patches() {
        return DBus.SessionBus.messages.filter(message => message.member === "UpdateSettings").map(message => JSON.parse(message.arguments[0]));
    }

    function init() {
        PreviewConfig.appliedSettings = null;
        client = clientComponent.createObject(suite) as DaemonClient;
        tryVerify(() => client.settings !== null, settleMs);
        DBus.SessionBus.messages = [];
    }

    function cleanup() {
        client.destroy();
        client = null;
    }

    function test_sends_only_changed_fields_in_order() {
        client.updateSettings({
            display: {
                value_mode: "used"
            }
        });
        client.updateSettings({
            refresh_interval_secs: 600
        });
        compare(members(), ["UpdateSettings"]);
        compare(DBus.SessionBus.messages[0].signature, "(s)");
        tryCompare(suite, "sentPatches", 2, settleMs);
        compare(patches(), [
            {
                display: {
                    value_mode: "used"
                }
            },
            {
                refresh_interval_secs: 600
            }
        ]);
        verify(!members().includes("SetSettings"));
        tryVerify(() => members().includes("GetSettings"), settleMs);
    }

    function test_applies_patches_optimistically() {
        client.updateSettings({
            display: {
                reset_format: "exact"
            }
        });
        compare(client.settings.display.resetFormat, "exact");
        compare(client.settings.display.valueMode, "left");
        tryVerify(() => !client.patchQueue.busy, settleMs);
        tryVerify(() => members().includes("GetSettings"), settleMs);
        wait(50);
        compare(client.settings.display.resetFormat, "exact");
    }

    Component {
        id: clientComponent

        DaemonClient {
            trackSettings: true
        }
    }
}
