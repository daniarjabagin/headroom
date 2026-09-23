import QtQuick
import org.kde.kirigami as Kirigami
import "logic/Providers.js" as Providers
import "logic/Tokens.js" as Tokens

Kirigami.Icon {
    required property string provider

    implicitWidth: Kirigami.Units.iconSizes.small
    implicitHeight: Kirigami.Units.iconSizes.small
    source: Qt.resolvedUrl(`../icons/${Providers.iconFile(provider)}`)
    isMask: Providers.isTinted(provider)
    color: Tokens.secondaryText(Kirigami.Theme)
}
