import QtQuick
import QtQuick.Window
import org.kde.kirigami as Kirigami
import org.kde.plasma.plasmoid
import headroom.preview

Window {
    id: preview

    readonly property var args: Qt.application.arguments
    readonly property string outPrefix: option("--out", "")
    readonly property int settleMs: 700
    property bool configured: false
    property int step: 0
    property var shots: []

    function option(name, fallback) {
        const index = args.indexOf(name);
        return index >= 0 && index + 1 < args.length ? args[index + 1] : fallback;
    }

    function flag(name) {
        return args.includes(name);
    }

    function findItems(item, predicate, found) {
        if (predicate(item))
            found.push(item);
        for (const child of item.children)
            findItems(child, predicate, found);
        return found;
    }

    function applyInteractions() {
        if (flag("--expand"))
            findItems(stage, item => typeof item.expandToggled === "function" && item.account !== undefined, []).forEach(section => section.expandToggled(section.account.id));
        if (flag("--menu"))
            findItems(stage, item => item.text === "Options" && typeof item.clicked === "function", []).forEach(button => button.clicked());
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

    function advance() {
        step += 1;
        if (step === 1) {
            applyInteractions();
            settle.restart();
        } else if (step === 2) {
            capture("light", () => {
                Kirigami.Theme.dark = true;
                settle.restart();
            });
        } else if (step === 3) {
            capture("dark", () => {
                composite.visible = true;
                settle.restart();
            });
        } else if (step === 4) {
            composite.grabToImage(result => {
                result.saveToFile(`${outPrefix}-preview.png`);
                console.info("saved", `${outPrefix}-preview.png`);
                Qt.quit();
            });
        }
    }

    width: Math.max(stage.implicitWidth, composite.visible ? composite.implicitWidth : 0)
    height: Math.max(stage.implicitHeight, composite.visible ? composite.implicitHeight : 0)
    visible: true
    color: Kirigami.Theme.dark ? "#0e1013" : "#cdd3db"
    Component.onCompleted: {
        PreviewConfig.scenario = option("--scenario", "ready");
        const statePath = option("--state", "");
        if (statePath !== "")
            PreviewConfig.statePath = Qt.resolvedUrl(`file://${statePath}`);
        Plasmoid.configuration = {
            showPercentage: true,
            alwaysShowPacing: flag("--pacing")
        };
        configured = true;
        if (outPrefix !== "")
            settle.start();
    }

    Row {
        id: stage

        padding: Kirigami.Units.gridUnit
        spacing: Kirigami.Units.gridUnit

        Loader {
            active: preview.configured
            source: "../../package/contents/ui/main.qml"
        }

        PanelSamples {}
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
