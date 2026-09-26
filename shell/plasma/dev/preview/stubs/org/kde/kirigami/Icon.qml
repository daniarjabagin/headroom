import QtQuick
import QtQuick.Window

Item {
    id: icon

    property var source: ""
    property bool isMask: false
    property color color: "black"
    readonly property var themeIcons: ({
            "arrow-down": "ui/pan-down-symbolic.svg",
            "arrow-up": "ui/pan-up-symbolic.svg",
            "go-up": "actions/go-up-symbolic.svg",
            "go-down": "actions/go-down-symbolic.svg",
            "dialog-warning": "status/dialog-warning-symbolic.svg",
            "dialog-error": "status/dialog-error-symbolic.svg",
            "documentinfo": "actions/help-about-symbolic.svg",
            "view-refresh": "actions/view-refresh-symbolic.svg",
            "configure": "legacy/emblem-system-symbolic.svg",
            "im-user": "status/avatar-default-symbolic.svg",
            "handle-sort": "ui/list-drag-handle-symbolic.svg",
            "list-add": "actions/list-add-symbolic.svg",
            "edit-delete": "actions/edit-delete-symbolic.svg",
            "utilities-terminal": "legacy/utilities-terminal-symbolic.svg",
            "application-x-executable": "legacy/system-users-symbolic.svg",
            "system-software-update": "status/software-update-available-symbolic.svg",
            "checkmark": "actions/object-select-symbolic.svg",
            "starred-symbolic": "status/starred-symbolic.svg",
            "non-starred-symbolic": "status/non-starred-symbolic.svg",
            "edit-copy": "actions/edit-copy-symbolic.svg",
            "folder-open": "status/folder-open-symbolic.svg",
            "folder": "places/folder-symbolic.svg",
            "arrow-right": "ui/pan-end-symbolic.svg",
            "network-connect": "places/network-server-symbolic.svg",
            "link": "actions/insert-link-symbolic.svg",
            "view-statistics": "status/network-cellular-signal-excellent-symbolic.svg",
            "view-hidden": "actions/view-conceal-symbolic.svg",
            "document-share": "actions/send-to-symbolic.svg"
        })
    readonly property string url: {
        const value = String(source);
        if (value.includes("/"))
            return value;
        const file = themeIcons[value];
        return file ? `file:///usr/share/icons/Adwaita/symbolic/${file}` : "";
    }

    implicitWidth: 16
    implicitHeight: 16

    Canvas {
        id: canvas

        readonly property real ratio: Screen.devicePixelRatio

        width: icon.width * ratio
        height: icon.height * ratio
        scale: 1 / ratio
        transformOrigin: Item.TopLeft
        onImageLoaded: requestPaint()
        onWidthChanged: requestPaint()
        onHeightChanged: requestPaint()
        onPaint: {
            const context = getContext("2d");
            context.reset();
            if (icon.url === "" || !isImageLoaded(icon.url))
                return;
            context.drawImage(icon.url, 0, 0, width, height);
            if (!icon.isMask)
                return;
            context.globalCompositeOperation = "source-in";
            context.fillStyle = icon.color;
            context.fillRect(0, 0, width, height);
        }
        Component.onCompleted: {
            if (icon.url !== "")
                loadImage(icon.url);
        }
    }

    onUrlChanged: {
        if (url !== "")
            canvas.loadImage(url);
        canvas.requestPaint();
    }
    onColorChanged: canvas.requestPaint()
}
