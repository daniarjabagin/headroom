import XCTest

@testable import HeadroomKit

final class HotKeyChordTests: XCTestCase {
    func testMapsGtkModifiersToCarbonFlags() throws {
        let chord = try XCTUnwrap(HotKeyChord.parse("<Super><Alt>u"))
        XCTAssertEqual(chord.modifiers, [.command, .option])
        XCTAssertEqual(chord.carbonModifiers, 0x0100 | 0x0800)
        XCTAssertEqual(chord.carbonKeyCode, 0x20)
    }

    func testEveryModifierSpelling() {
        let cases: [(String, HotKeyModifiers)] = [
            ("<Super>h", .command), ("<Meta>h", .command), ("<Primary>h", .command),
            ("<Control>h", .control), ("<Ctrl>h", .control), ("<ctl>h", .control),
            ("<Alt>h", .option), ("<Mod1>h", .option), ("<Shift>h", .shift),
            ("<Control><Shift><Alt><Super>h", [.control, .shift, .option, .command]),
        ]
        for (text, modifiers) in cases {
            XCTAssertEqual(HotKeyChord.parse(text)?.modifiers, modifiers, text)
        }
    }

    func testCarbonFlagValues() {
        XCTAssertEqual(HotKeyModifiers.command.rawValue, 256)
        XCTAssertEqual(HotKeyModifiers.shift.rawValue, 512)
        XCTAssertEqual(HotKeyModifiers.option.rawValue, 2048)
        XCTAssertEqual(HotKeyModifiers.control.rawValue, 4096)
    }

    func testKeyNamesAreCaseInsensitive() {
        let cases: [(String, UInt32)] = [
            ("<Super>A", 0x00), ("<Super>a", 0x00), ("<Super>0", 0x1D), ("<Super>9", 0x19),
            ("<Super>space", 0x31), ("<Super>Return", 0x24), ("<Super>Escape", 0x35), ("<Super>BackSpace", 0x33),
            ("<Super>Delete", 0x75), ("<Super>Page_Up", 0x74), ("<Super>Left", 0x7B), ("<Super>minus", 0x1B),
            ("<Super>comma", 0x2B), ("<Super>grave", 0x32), ("<Super>F12", 0x6F),
        ]
        for (text, code) in cases {
            XCTAssertEqual(HotKeyChord.parse(text)?.carbonKeyCode, code, text)
        }
    }

    func testFunctionKeysNeedNoModifier() {
        XCTAssertEqual(HotKeyChord.parse("F5")?.carbonKeyCode, 0x60)
        XCTAssertEqual(HotKeyChord.parse("F5")?.modifiers, [])
    }

    func testRejectsWhatCannotBeRegistered() {
        for text in ["", "u", "<Super>", "<Hyper>u", "<Super>u<", "<Super>euro", "<>u", "<Super", "<Super>F21"] {
            XCTAssertNil(HotKeyChord.parse(text), text)
        }
    }
}
