import QtQuick
import org.kde.kirigami as Kirigami
import "logic/PanelLayout.js" as PanelLayout

SequentialAnimation {
    id: pulse

    required property Item subject

    loops: Animation.Infinite
    onStopped: subject.opacity = 1

    NumberAnimation {
        target: pulse.subject
        property: "opacity"
        to: PanelLayout.pulseOpacity()
        duration: PanelLayout.pulsePeriod(Kirigami.Units) / 2
        easing.type: Easing.InOutSine
    }

    NumberAnimation {
        target: pulse.subject
        property: "opacity"
        to: 1
        duration: PanelLayout.pulsePeriod(Kirigami.Units) / 2
        easing.type: Easing.InOutSine
    }
}
