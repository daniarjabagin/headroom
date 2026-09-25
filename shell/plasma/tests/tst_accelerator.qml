import QtQuick
import QtTest
import "../package/contents/ui/logic/Accelerator.js" as Accelerator

TestCase {
    name: "Accelerator"

    function test_key_sequences_data() {
        return [
            {
                tag: "super letter",
                accelerator: "<Super>u",
                sequence: "Meta+U"
            },
            {
                tag: "control alt",
                accelerator: "<Control><Alt>h",
                sequence: "Ctrl+Alt+H"
            },
            {
                tag: "modifier order is canonical",
                accelerator: "<Shift><Super><Primary>k",
                sequence: "Ctrl+Shift+Meta+K"
            },
            {
                tag: "aliases and case",
                accelerator: "<ctrl><MOD1><Mod4>F12",
                sequence: "Ctrl+Alt+Meta+F12"
            },
            {
                tag: "duplicate modifiers",
                accelerator: "<Super><Meta>1",
                sequence: "Meta+1"
            },
            {
                tag: "named key",
                accelerator: "<Super>Page_Down",
                sequence: "Meta+PgDown"
            },
            {
                tag: "punctuation key",
                accelerator: "<Control>bracketleft",
                sequence: "Ctrl+["
            },
            {
                tag: "no modifier",
                accelerator: "F5",
                sequence: "F5"
            },
            {
                tag: "empty disables",
                accelerator: "",
                sequence: ""
            },
            {
                tag: "unknown modifier",
                accelerator: "<Hyper>u",
                sequence: ""
            },
            {
                tag: "unknown key",
                accelerator: "<Super>XF86AudioPlay",
                sequence: ""
            },
            {
                tag: "prototype names are not keys",
                accelerator: "<constructor>constructor",
                sequence: ""
            },
            {
                tag: "missing key",
                accelerator: "<Super>",
                sequence: ""
            },
            {
                tag: "function key out of range",
                accelerator: "F36",
                sequence: ""
            },
            {
                tag: "not a string",
                accelerator: null,
                sequence: ""
            }
        ];
    }

    function test_key_sequences(data) {
        compare(Accelerator.keySequence(data.accelerator), data.sequence);
    }

    function test_next_shortcut_applies_changes_only() {
        compare(Accelerator.nextShortcut("<Super>u", true, null), "Meta+U");
        compare(Accelerator.nextShortcut("<Super>u", true, "Meta+U"), null);
        compare(Accelerator.nextShortcut("", true, "Meta+U"), "");
        compare(Accelerator.nextShortcut("", true, ""), null);
    }

    function test_next_shortcut_waits_for_a_capable_daemon() {
        compare(Accelerator.nextShortcut("<Super>u", false, null), null);
        compare(Accelerator.nextShortcut("", false, "Meta+U"), null);
    }
}
