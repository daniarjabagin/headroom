import QtQuick
import org.kde.plasma.configuration
import "../ui/logic/I18n.js" as I18n

ConfigModel {
    id: model

    readonly property string lang: I18n.resolve("system", Qt.locale().name)

    ConfigCategory {
        name: I18n.tr(model.lang, "General")
        icon: "configure"
        source: "ConfigGeneral.qml"
    }

    ConfigCategory {
        name: I18n.tr(model.lang, "Accounts")
        icon: "system-users"
        source: "ConfigAccounts.qml"
    }

    ConfigCategory {
        name: I18n.tr(model.lang, "Notifications")
        icon: "preferences-desktop-notification"
        source: "ConfigNotifications.qml"
    }

    ConfigCategory {
        name: I18n.tr(model.lang, "Advanced")
        icon: "preferences-other"
        source: "ConfigAdvanced.qml"
    }
}
