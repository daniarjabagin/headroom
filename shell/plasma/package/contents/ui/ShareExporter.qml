pragma ComponentBehavior: Bound

import QtCore
import QtQuick
import QtQuick.Window
import org.kde.kirigami as Kirigami
import "logic/Share.js" as Share

Item {
    id: exporter

    required property var display
    required property bool dark
    property url picturesUrl: StandardPaths.writableLocation(StandardPaths.PicturesLocation)
    property var pending: null
    property var grabbed: null
    readonly property var metrics: Share.layout()
    readonly property bool busy: pending !== null
    readonly property int settleMs: 100

    signal saved(string folder)
    signal failed
    signal copied

    function share(shared, members) {
        const folder = Share.folderPath(picturesUrl);
        if (busy || folder === "" || shared.hero === null) {
            failed();
            return;
        }
        pending = {
            shared,
            members,
            folder,
            path: `${folder}/${Share.fileName(shared.provider, new Date())}`
        };
        cardLoader.active = true;
        settle.restart();
    }

    function grab() {
        const ratio = Screen.devicePixelRatio > 0 ? Screen.devicePixelRatio : 1;
        const size = Qt.size(metrics.width * metrics.scale / ratio, metrics.height * metrics.scale / ratio);
        if (!(cardLoader.item as ShareCard).grabToImage(result => exporter.save(result), size))
            finish(false);
    }

    function save(result) {
        if (result.saveToFile(pending.path)) {
            finish(true);
            return;
        }
        grabbed = result;
        folderMaker.active = true;
        (folderMaker.item as CommandRunner).run(Share.folderCommand(pending.folder));
    }

    function folderMade() {
        finish(grabbed !== null && grabbed.saveToFile(pending.path));
    }

    function finish(ok) {
        const folder = pending.folder;
        cardLoader.active = false;
        folderMaker.active = false;
        pending = null;
        grabbed = null;
        if (ok)
            saved(folder);
        else
            failed();
    }

    function copy(text) {
        clipboard.text = text;
        clipboard.selectAll();
        clipboard.copy();
        copied();
    }

    Loader {
        id: cardLoader

        x: exporter.width + Kirigami.Units.gridUnit
        active: false

        sourceComponent: ShareCard {
            shared: exporter.pending.shared
            members: exporter.pending.members
            display: exporter.display
            dark: exporter.dark
        }
    }

    Loader {
        id: folderMaker

        active: false

        sourceComponent: CommandRunner {
            onExited: Qt.callLater(exporter.folderMade)
        }
    }

    Timer {
        id: settle

        interval: exporter.settleMs
        onTriggered: exporter.grab()
    }

    TextEdit {
        id: clipboard

        visible: false
    }
}
