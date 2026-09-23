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
            "dialog-warning": "status/dialog-warning-symbolic.svg",
            "dialog-error": "status/dialog-error-symbolic.svg",
            "documentinfo": "actions/help-about-symbolic.svg",
            "view-refresh": "actions/view-refresh-symbolic.svg",
            "configure": "legacy/emblem-system-symbolic.svg",
            "im-user": "status/avatar-default-symbolic.svg",
            "application-x-executable": "legacy/system-users-symbolic.svg"
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
