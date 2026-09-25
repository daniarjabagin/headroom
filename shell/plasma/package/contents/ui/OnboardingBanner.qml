import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/I18n.js" as I18n
import "logic/Metrics.js" as Metrics
import "logic/Onboarding.js" as Onboarding
import "logic/Tokens.js" as Tokens

Rectangle {
    id: banner

    required property var settings
    required property var snapshot
    required property bool capable
    required property string lang

    signal reviewRequested
    signal dismissed

    objectName: "onboardingBanner"
    visible: Onboarding.pending(settings, capable)
    Layout.fillWidth: true
    Layout.bottomMargin: Metrics.sectionGap(Kirigami.Units)
    implicitHeight: content.implicitHeight + Metrics.cardPadding(Kirigami.Units) * 2
    radius: Metrics.cardRadius(Kirigami.Units)
    color: Tokens.updateFill(Kirigami.Theme)

    ColumnLayout {
        id: content

        anchors.fill: parent
        anchors.margins: Metrics.cardPadding(Kirigami.Units)
        spacing: Kirigami.Units.smallSpacing

        RowLayout {
            spacing: Kirigami.Units.largeSpacing

            Kirigami.Icon {
                Layout.alignment: Qt.AlignTop
                implicitWidth: Kirigami.Units.iconSizes.smallMedium
                implicitHeight: implicitWidth
                source: Qt.resolvedUrl("../icons/headroom-symbolic.svg")
                isMask: true
                color: Kirigami.Theme.highlightColor
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: Math.round(Kirigami.Units.smallSpacing / 2)

                TextLabel {
                    objectName: "onboardingTitle"
                    Layout.fillWidth: true
                    role: "label"
                    wrapMode: Text.Wrap
                    text: Onboarding.title(banner.lang, banner.snapshot)
                }

                TextLabel {
                    Layout.fillWidth: true
                    role: "caption"
                    emphasis: "secondary"
                    wrapMode: Text.Wrap
                    text: Onboarding.detail(banner.lang)
                }
            }
        }

        RowLayout {
            Layout.alignment: Qt.AlignRight
            spacing: Kirigami.Units.smallSpacing

            SmallButton {
                objectName: "onboardingReview"
                text: I18n.tr(banner.lang, "Review")
                onClicked: banner.reviewRequested()
            }

            SmallButton {
                objectName: "onboardingDone"
                primary: true
                text: I18n.tr(banner.lang, "Got it")
                onClicked: banner.dismissed()
            }
        }
    }
}
