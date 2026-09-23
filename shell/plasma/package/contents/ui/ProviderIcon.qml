import QtQuick
import org.kde.kirigami as Kirigami
import "logic/Providers.js" as Providers
import "logic/Tokens.js" as Tokens

Kirigami.Icon {
    required property string provider
    readonly property var info: Providers.providerInfo(provider)

    implicitWidth: Kirigami.Units.iconSizes.small
    implicitHeight: Kirigami.Units.iconSizes.small
    source: info.icon ? Qt.resolvedUrl(`../icons/${info.icon}`) : "application-x-executable"
    isMask: info.tinted
    color: Tokens.secondaryText(Kirigami.Theme)
}
