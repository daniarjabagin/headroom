import QtQuick
import QtQuick.Window
import org.kde.kirigami as Kirigami
import headroom.preview

Window {
    id: preview

    readonly property var args: Qt.application.arguments
    readonly property string outPrefix: option("--out", "")
    readonly property string configPage: option("--config", "")
    readonly property bool forcedLook: option("--theme", "") !== "" || flag("--translucent") || flag("--dark")
    readonly property int settleMs: 900
    readonly property var frameDelays: [40, 120, 220, 360]
    property bool configured: false
    property var steps: []
    property var shots: []

    function option(name, fallback) {
        const index = args.lastIndexOf(name);
        return index >= 0 && index + 1 < args.length ? args[index + 1] : fallback;
    }

    function flag(name) {
        return args.includes(name);
    }

    function findObjects(object, predicate, found) {
        if (predicate(object))
            found.push(object);
        for (const child of object.data ?? [])
            findObjects(child, predicate, found);
        return found;
    }

    function displayPatch() {
        const patch = {};
        const pairs = [["--lang", "language"], ["--theme", "theme"], ["--label", "panel_label"]];
        for (const [name, key] of pairs)
            if (option(name, "") !== "")
                patch[key] = option(name, "");
        if (flag("--used"))
            patch.value_mode = "used";
        if (flag("--exact"))
            patch.reset_format = "exact";
        if (flag("--translucent"))
            patch.translucent = true;
        return patch;
    }

    function applyInteractions() {
        if (flag("--expand"))
            findObjects(stage, item => typeof item.expandToggled === "function" && item.account !== undefined, []).forEach(section => section.gap === undefined ? section.expandToggled() : section.expandToggled(section.account.id));
        if (option("--pick", "") !== "")
            findObjects(stage, item => item.selectedProvider !== undefined && item.chosenProvider !== undefined, []).forEach(page => page.selectedProvider = option("--pick", ""));
        if (flag("--update"))
            findObjects(stage, item => item.objectName === "updateRow", []).forEach(row => row.activate());
        if (flag("--tooltip"))
            findObjects(stage, item => item.breakdown !== undefined && item.breakdown !== null && item.tip !== undefined, []).slice(0, 1).forEach(handler => handler.tip.visible = true);
        if (flag("--popover"))
            findObjects(stage, item => item.notes !== undefined && item.rows !== undefined && item.tip !== undefined, []).slice(0, 1).forEach(handler => handler.tip.visible = true);
        if (flag("--unit-menu"))
            findObjects(stage, item => item.objectName === "unitTitle", []).forEach(title => title.clicked());
        if (flag("--menu"))
            findObjects(stage, item => typeof item.openMenu === "function", []).slice(1, 2).forEach(header => header.openMenu());
        if (option("--sheen", "") !== "")
            findObjects(stage, item => item.glowColor !== undefined && item.sheen !== undefined, []).forEach(meter => meter.sheen = Number(option("--sheen", "")));
        if (flag("--unfold"))
            findObjects(stage, item => typeof item.setFolded === "function", []).forEach(dashboard => dashboard.setFolded(true));
        if (option("--share", "") !== "") {
            findObjects(stage, item => item.picturesUrl !== undefined, []).forEach(exporter => exporter.picturesUrl = `file://${option("--share", "")}`);
            findObjects(stage, item => typeof item.shareRequested === "function" && item.windows !== undefined && item.windows.length > 0, []).slice(0, 1).forEach(section => section.shareRequested());
        }
    }

    function capture(name, next) {
        preview.contentItem.grabToImage(result => {
            const path = `${outPrefix}-${name}.png`;
            result.saveToFile(path);
            shots = shots.concat([path]);
            console.info("saved", path);
            next();
        });
    }

    function replayOpen() {
        findObjects(stage, item => typeof item.playOpen === "function", []).forEach(full => full.playOpen());
    }

    function frameSteps() {
        const list = [() => {
                replayOpen();
                wait(frameDelays[0]);
            }];
        frameDelays.forEach((delay, index) => list.push(() => capture(`frame${index + 1}`, () => wait(index + 1 < frameDelays.length ? frameDelays[index + 1] - delay : 1))));
        return list;
    }

    function plan() {
        const list = [() => {
                applyInteractions();
                wait(settleMs);
            }];
        if (flag("--frames"))
            return list.concat(frameSteps()).concat([() => Qt.quit()]);
        if (forcedLook || configPage !== "")
            return list.concat([() => capture("shot", () => Qt.quit())]);
        return list.concat([() => capture("light", () => {
                    Kirigami.Theme.dark = true;
                    wait(settleMs);
                }), () => capture("dark", () => {
                    composite.visible = true;
                    wait(settleMs);
                }), () => composite.grabToImage(result => {
                    result.saveToFile(`${outPrefix}-preview.png`);
                    console.info("saved", `${outPrefix}-preview.png`);
                    Qt.quit();
                })]);
    }

    function wait(ms) {
        settle.interval = ms;
        settle.restart();
    }

    function advance() {
        const next = steps.shift();
        if (next)
            next();
    }

    width: Math.max(stage.implicitWidth, composite.visible ? composite.implicitWidth : 0)
    height: Math.max(stage.implicitHeight, composite.visible ? composite.implicitHeight : 0)
    visible: true
    color: Kirigami.Theme.dark ? "#0e1013" : "#cdd3db"
    Component.onCompleted: {
        PreviewConfig.scenario = option("--scenario", "ready");
        PreviewConfig.displayPatch = displayPatch();
        PreviewConfig.wallpaper = flag("--translucent");
        Kirigami.Theme.dark = flag("--dark");
        const statePath = option("--state", "");
        if (statePath !== "")
            PreviewConfig.statePath = Qt.resolvedUrl(`file://${statePath}`);
        configured = true;
        steps = plan();
        if (outPrefix !== "")
            wait(settleMs);
    }

    Rectangle {
        visible: PreviewConfig.wallpaper
        anchors.fill: parent
        rotation: 0

        gradient: Gradient {
            GradientStop {
                position: 0
                color: "#5b6cff"
            }

            GradientStop {
                position: 0.5
                color: "#e0609a"
            }

            GradientStop {
                position: 1
                color: "#ffb347"
            }
        }
    }

    Row {
        id: stage

        padding: Kirigami.Units.gridUnit
        spacing: Kirigami.Units.gridUnit

        Loader {
            active: preview.configured && preview.configPage === ""
            visible: active
            source: "../../package/contents/ui/main.qml"
        }

        PanelSamples {
            visible: preview.configPage === ""
        }

        Rectangle {
            visible: preview.configPage !== ""
            width: configLoader.item ? configLoader.item.implicitWidth : 0
            height: configLoader.item ? configLoader.item.implicitHeight : 0
            radius: PreviewConfig.dialogRadius
            color: Kirigami.Theme.backgroundColor

            Loader {
                id: configLoader

                anchors.fill: parent
                active: preview.configured && preview.configPage !== ""
                source: `../../package/contents/ui/Config${preview.configPage.charAt(0).toUpperCase()}${preview.configPage.slice(1)}.qml`
            }
        }
    }

    Rectangle {
        id: composite

        visible: false
        implicitWidth: shotsRow.implicitWidth
        implicitHeight: shotsRow.implicitHeight
        color: "#000000"

        Row {
            id: shotsRow

            Repeater {
                model: preview.shots

                Image {
                    required property string modelData

                    width: sourceSize.width / Screen.devicePixelRatio
                    height: sourceSize.height / Screen.devicePixelRatio
                    source: `file://${modelData}`
                    cache: false
                }
            }
        }
    }

    Timer {
        id: settle

        interval: preview.settleMs
        onTriggered: preview.advance()
    }
}
