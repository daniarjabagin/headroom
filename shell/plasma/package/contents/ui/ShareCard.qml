pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "logic/Providers.js" as Providers
import "logic/Share.js" as Share
import "logic/Tokens.js" as Tokens

Rectangle {
    id: share

    required property var shared
    required property var members
    required property var display
    required property bool dark
    readonly property var colors: Share.palette(dark)
    readonly property var metrics: Share.layout()

    component ShareText: Text {
        property int size: share.metrics.body
        property bool dim: false

        color: dim ? share.colors.dim : share.colors.foreground
        font.family: Kirigami.Theme.defaultFont.family
        font.pixelSize: size
        font.features: {
            "tnum": 1
        }
        textFormat: Text.PlainText
    }

    component Pill: Rectangle {
        required property real fraction
        property color fill: share.colors.foreground
        property real tick: -1

        radius: height / 2
        color: share.colors.line

        Rectangle {
            height: parent.height
            radius: height / 2
            width: parent.fraction > 0 ? Math.max(height, parent.width * parent.fraction) : 0
            color: parent.fill
        }

        Rectangle {
            visible: parent.tick >= 0
            width: 2
            height: parent.height + 4
            y: -2
            x: Math.min(parent.width - width, Math.max(0, parent.width * parent.tick - width / 2))
            radius: 1
            color: share.colors.dim
        }
    }

    objectName: "shareCard"
    width: metrics.width
    height: metrics.height
    color: colors.background

    ColumnLayout {
        anchors.fill: parent
        anchors.topMargin: share.metrics.padTop
        anchors.leftMargin: share.metrics.padSide
        anchors.rightMargin: share.metrics.padSide
        anchors.bottomMargin: share.metrics.padBottom
        spacing: 0

        RowLayout {
            Layout.bottomMargin: share.metrics.padBottom - share.metrics.small
            spacing: share.metrics.small

            Kirigami.Icon {
                implicitWidth: share.metrics.logo + share.metrics.small
                implicitHeight: implicitWidth
                source: Qt.resolvedUrl("../icons/headroom-symbolic.svg")
                isMask: true
                color: share.colors.foreground
            }

            ShareText {
                size: share.metrics.logo
                font.weight: Font.Bold
                text: "headroom"
            }

            Item {
                Layout.fillWidth: true
            }

            ShareText {
                size: share.metrics.tiny
                dim: true
                font.letterSpacing: share.metrics.tiny * 0.08
                text: share.shared.title
            }
        }

        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 1
            color: share.colors.line
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: share.metrics.gap

            ColumnLayout {
                Layout.fillWidth: true
                Layout.alignment: Qt.AlignVCenter
                spacing: share.metrics.small

                RowLayout {
                    spacing: share.metrics.small

                    Kirigami.Icon {
                        implicitWidth: share.metrics.name + 3
                        implicitHeight: implicitWidth
                        source: Qt.resolvedUrl(`../icons/${Providers.iconFile(share.shared.provider)}`)
                        isMask: Providers.isTinted(share.shared.provider)
                        color: share.colors.foreground
                    }

                    ShareText {
                        size: share.metrics.name
                        font.weight: Font.DemiBold
                        text: share.shared.providerName
                    }

                    ShareText {
                        Layout.fillWidth: true
                        visible: share.shared.plan !== ""
                        size: share.metrics.name
                        dim: true
                        elide: Text.ElideRight
                        text: `· ${share.shared.plan}`
                    }
                }

                RowLayout {
                    visible: share.shared.hero !== null
                    spacing: share.metrics.small

                    ShareText {
                        objectName: "shareHero"
                        Layout.alignment: Qt.AlignBaseline
                        size: share.metrics.hero
                        font.weight: Font.Bold
                        font.letterSpacing: -share.metrics.hero * 0.03
                        text: share.shared.hero?.value ?? ""
                    }

                    ShareText {
                        Layout.alignment: Qt.AlignBaseline
                        size: share.metrics.heroSuffix
                        font.weight: Font.DemiBold
                        text: share.shared.hero?.suffix ?? ""
                    }
                }

                ShareText {
                    Layout.fillWidth: true
                    dim: true
                    elide: Text.ElideRight
                    text: share.shared.hero?.sub ?? ""
                }

                Row {
                    Layout.topMargin: share.metrics.small
                    spacing: share.metrics.pillGap

                    Repeater {
                        id: heroPills

                        model: Share.fractions(share.shared.hero?.window ?? null, share.display.valueMode)

                        Pill {
                            required property var modelData

                            width: (share.metrics.heroMeter - share.metrics.pillGap * (heroPills.count - 1)) / Math.max(1, heroPills.count)
                            height: share.metrics.pill
                            fraction: modelData
                        }
                    }
                }
            }

            Rectangle {
                Layout.alignment: Qt.AlignVCenter
                implicitWidth: share.metrics.cardWidth
                implicitHeight: rows.implicitHeight + share.metrics.cardPad * 2
                radius: share.metrics.cardRadius
                color: share.colors.card

                ColumnLayout {
                    id: rows

                    anchors.fill: parent
                    anchors.margins: share.metrics.cardPad
                    spacing: share.metrics.cardPad

                    Repeater {
                        model: Share.shownRows(share.shared)

                        ColumnLayout {
                            id: windowRow

                            required property var modelData
                            readonly property var segments: Share.meterSegments(modelData.window, share.members, share.display)

                            Layout.fillWidth: true
                            spacing: share.metrics.small

                            ShareText {
                                size: share.metrics.name
                                font.weight: Font.DemiBold
                                text: windowRow.modelData.label
                            }

                            Row {
                                id: meterRow

                                Layout.fillWidth: true
                                spacing: share.metrics.pillGap

                                Repeater {
                                    model: windowRow.segments

                                    Pill {
                                        required property var modelData

                                        width: (meterRow.width - share.metrics.pillGap * (windowRow.segments.length - 1)) / windowRow.segments.length
                                        height: share.metrics.meter
                                        fraction: modelData.fraction
                                        tick: modelData.tick ?? -1
                                        fill: Tokens.toneColor(Kirigami.Theme, modelData.tone === "neutral" ? "good" : modelData.tone)
                                    }
                                }
                            }

                            RowLayout {
                                spacing: share.metrics.small

                                ShareText {
                                    Layout.fillWidth: true
                                    elide: Text.ElideRight
                                    text: windowRow.modelData.reading
                                }

                                ShareText {
                                    dim: true
                                    text: windowRow.modelData.reset
                                }
                            }
                        }
                    }
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 1
            color: share.colors.line
        }

        RowLayout {
            Layout.topMargin: share.metrics.padBottom - share.metrics.small * 2
            spacing: share.metrics.small

            Rectangle {
                implicitWidth: share.metrics.small + 2
                implicitHeight: implicitWidth
                color: share.colors.accent

                transform: Matrix4x4 {
                    matrix: Qt.matrix4x4(1, -0.29, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1)
                }
            }

            ShareText {
                Layout.fillWidth: true
                size: share.metrics.tiny
                dim: true
                text: "Know what's left."
            }

            ShareText {
                size: share.metrics.tiny
                dim: true
                text: "headroom"
            }
        }
    }
}
