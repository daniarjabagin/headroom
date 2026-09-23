import QtQuick
import QtTest
import "../package/contents/ui/logic/Motion.js" as Motion

TestCase {
    name: "Motion"

    function test_spin_turn_is_seven_long_durations() {
        compare(Motion.spinDuration({
            longDuration: 200
        }), 1400);
    }

    function test_next_turn_data() {
        return [
            {
                tag: "rest",
                angle: 0,
                target: 0
            },
            {
                tag: "ramp",
                angle: 90,
                target: 360
            },
            {
                tag: "exact",
                angle: 360,
                target: 360
            },
            {
                tag: "second turn",
                angle: 361,
                target: 720
            }
        ];
    }

    function test_next_turn(data) {
        compare(Motion.nextTurn(data.angle), data.target);
    }

    function test_settle_keeps_linear_speed_at_start() {
        compare(Motion.settleDuration(180, 1400), 1400);
        compare(Motion.settleDuration(45, 1400), 350);
        compare(Motion.settleDuration(0, 1400), 0);
        compare(Motion.settleDuration(-5, 1400), 0);
    }

    function test_ramp_reaches_linear_speed_after_half_turn() {
        compare(Motion.spinRampAngle() * 2, Motion.turnAngle());
    }
}
