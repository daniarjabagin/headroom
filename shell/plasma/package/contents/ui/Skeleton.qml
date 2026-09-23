pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/I18n.js" as I18n
import "logic/Metrics.js" as Metrics
import "logic/Motion.js" as Motion
import "logic/Tokens.js" as Tokens

Item {
    id: skeleton

    required property string lang
    required property bool reducedMotion
    readonly property int sectionCount: 2
    readonly property int rowCount: 2
    property real phase: 0.5

    function glow(position) {
        return Math.max(0, 1 - Math.abs(position - phase) * 3);
    }

    Layout.fillWidth: true
    implicitHeight: column.implicitHeight
    Accessible.name: I18n.tr(lang, "Loading…")

    ColumnLayout {
        id: column

        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        spacing: Metrics.sectionGap(Kirigami.Units)

        Repeater {
            model: skeleton.sectionCount

            ColumnLayout {
                Layout.fillWidth: true
                spacing: Kirigami.Units.smallSpacing

                RowLayout {
                    Layout.leftMargin: Metrics.headerInset(Kirigami.Units)
                    spacing: Kirigami.Units.mediumSpacing

                    Bone {
                        length: Kirigami.Units.iconSizes.small
                        thickness: Kirigami.Units.iconSizes.small
                        position: 0.05
                    }

                    Bone {
                        length: Kirigami.Units.gridUnit * 6
                        thickness: Kirigami.Units.largeSpacing + Kirigami.Units.smallSpacing
                        position: 0.25
                    }
                }

                Card {
                    Repeater {
                        model: skeleton.rowCount

                        ColumnLayout {
                            Layout.fillWidth: true
                            Layout.leftMargin: Metrics.rowInset(Kirigami.Units)
                            Layout.rightMargin: Metrics.rowInset(Kirigami.Units)
                            Layout.topMargin: Metrics.barRowPadding(Kirigami.Units)
                            Layout.bottomMargin: Metrics.barRowPadding(Kirigami.Units)
                            spacing: Kirigami.Units.mediumSpacing

                            Bone {
                                length: Kirigami.Units.gridUnit * 3.5
                                thickness: Kirigami.Units.largeSpacing + Kirigami.Units.smallSpacing / 2
                                position: 0.15
                            }

                            Bone {
                                Layout.fillWidth: true
                                thickness: Metrics.meterHeight(Kirigami.Units)
                                position: 0.5
                            }

                            RowLayout {
                                Bone {
                                    length: Kirigami.Units.gridUnit * 3
                                    thickness: Kirigami.Units.largeSpacing
                                    position: 0.15
                                }

                                Item {
                                    Layout.fillWidth: true
                                }

                                Bone {
                                    length: Kirigami.Units.gridUnit * 5
                                    thickness: Kirigami.Units.largeSpacing
                                    position: 0.85
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    SequentialAnimation on phase {
        running: skeleton.visible && Motion.enabled(Kirigami.Units, skeleton.reducedMotion)
        loops: Animation.Infinite

        NumberAnimation {
            from: -0.4
            to: 1.4
            duration: Motion.shimmerDuration(Kirigami.Units)
            easing.type: Easing.InOutSine
        }
    }

    component Bone: Rectangle {
        property real length: 0
        property real thickness: 0
        property real position: 0

        implicitWidth: length
        implicitHeight: thickness
        radius: Math.min(height / 2, Metrics.chipRadius(Kirigami.Units))
        color: Tokens.skeletonGlow(Kirigami.Theme, skeleton.glow(position))
    }
}
