import QtQuick
import QtTest
import org.kde.kirigami as Kirigami
import "../package/contents/ui"
import "../package/contents/ui/logic/Motion.js" as Motion
import "../package/contents/ui/logic/Tokens.js" as Tokens

TestCase {
    id: suite

    property Item target: null

    function init() {
        mouseMove(suite, suite.width - 1, suite.height - 1);
        target = createTemporaryObject(targetComponent, suite) as Item;
    }

    function pointer() {
        return (target as HoverTarget).pointer;
    }

    function test_hover_needs_real_pointer_motion() {
        mouseMove(target, 10, 10);
        verify(pointer().hovered);
        verify(!pointer().shown);
        mouseMove(target, 12, 12);
        verify(pointer().shown);
    }

    function test_hover_clears_when_pointer_leaves() {
        mouseMove(target, 10, 10);
        mouseMove(target, 12, 12);
        verify(pointer().shown);
        mouseMove(suite, suite.width - 1, suite.height - 1);
        verify(!pointer().hovered);
        verify(!pointer().shown);
    }

    function test_hover_follows_duration_and_reduced_motion() {
        compare(Motion.hoverDuration({
            longDuration: 200
        }), 200);
        compare(Motion.hoverDuration({
            longDuration: 0
        }), 0);
        verify(Motion.moved(Qt.point(1, 1), Qt.point(1, 2)));
        verify(!Motion.moved(Qt.point(3, 4), Qt.point(3, 4)));
    }

    function test_hover_tint_is_softer_in_light_theme() {
        const light = {
            backgroundColor: Qt.color("#ffffff"),
            textColor: Qt.color("#000000")
        };
        const dark = {
            backgroundColor: Qt.color("#000000"),
            textColor: Qt.color("#ffffff")
        };
        const lightShift = 1 - Tokens.hover(light).r;
        const darkShift = Tokens.hover(dark).r;
        verify(lightShift > 0);
        verify(lightShift < darkShift);
    }

    function test_hover_fill_matches_its_element_radius() {
        const fill = createTemporaryObject(fillComponent, suite);
        compare(fill.radius, Kirigami.Units.smallSpacing);
        compare(fill.opacity, 0);
        fill.shown = true;
        tryCompare(fill, "opacity", 1);
    }

    width: 200
    height: 200
    visible: true
    when: windowShown

    Component {
        id: targetComponent

        HoverTarget {}
    }

    component HoverTarget: Rectangle {
        readonly property alias pointer: hover

        width: 100
        height: 100

        PointerHover {
            id: hover
        }
    }

    Component {
        id: fillComponent

        HoverFill {
            width: 40
            height: 20
            radius: Kirigami.Units.smallSpacing
        }
    }
}
