import QtQuick
import QtQuick.Controls as QQC2
import org.kde.kcmutils as KCM
import org.kde.kirigami as Kirigami

KCM.SimpleKCM {
    property alias cfg_showPercentage: showPercentage.checked
    property alias cfg_alwaysShowPacing: alwaysShowPacing.checked

    Kirigami.FormLayout {
        QQC2.CheckBox {
            id: showPercentage

            Kirigami.FormData.label: "Panel:"
            text: "Show remaining percentage"
        }

        QQC2.CheckBox {
            id: alwaysShowPacing

            Kirigami.FormData.label: "Limits:"
            text: "Always show pacing"
        }
    }
}
