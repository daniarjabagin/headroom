import QtQuick
import org.kde.kirigami as Kirigami
import "logic/Motion.js" as Motion

Kirigami.Icon {
    id: icon

    property bool spinning: false
    property bool animated: true
    readonly property int turnMs: Motion.spinDuration(Kirigami.Units)
    readonly property bool turning: ramp.running || loop.running || settle.running

    function begin() {
        settle.stop();
        if (ramp.running || loop.running)
            return;
        if (rotation === 0)
            ramp.start();
        else
            runLoop(rotation % Motion.turnAngle());
    }

    function runLoop(from) {
        loop.from = from;
        loop.to = from + Motion.turnAngle();
        loop.start();
    }

    function end() {
        if (!ramp.running && !loop.running)
            return;
        ramp.stop();
        loop.stop();
        const target = Motion.nextTurn(rotation);
        settle.to = target;
        settle.duration = Motion.settleDuration(target - rotation, turnMs);
        settle.start();
    }

    function rest() {
        ramp.stop();
        loop.stop();
        settle.stop();
        rotation = 0;
    }

    function sync() {
        if (!animated)
            rest();
        else if (spinning)
            begin();
        else
            end();
    }

    onSpinningChanged: sync()
    onAnimatedChanged: sync()
    Component.onCompleted: sync()

    NumberAnimation {
        id: ramp

        target: icon
        property: "rotation"
        from: 0
        to: Motion.spinRampAngle()
        duration: icon.turnMs
        easing.type: Easing.InQuad
        onFinished: icon.runLoop(Motion.spinRampAngle())
    }

    NumberAnimation {
        id: loop

        target: icon
        property: "rotation"
        duration: icon.turnMs
        loops: Animation.Infinite
    }

    NumberAnimation {
        id: settle

        target: icon
        property: "rotation"
        easing.type: Easing.OutQuad
        onFinished: icon.rotation = 0
    }
}
