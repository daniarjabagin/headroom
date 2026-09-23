pragma Singleton

import QtQuick
import org.kde.plasma.core as PlasmaCore
import headroom.preview

QtObject {
    id: plasmoid

    property int status: PlasmaCore.Types.UnknownStatus
    property int formFactor: PlasmaCore.Types.Horizontal
    readonly property var metaData: ({
            version: JSON.parse(PreviewConfig.readFile(PreviewConfig.metadataPath)).KPlugin.Version
        })
    property string triggeredAction: ""

    function internalAction(name) {
        return {
            trigger: () => plasmoid.triggeredAction = name
        };
    }
}
