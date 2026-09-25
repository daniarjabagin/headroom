import XCTest

@testable import HeadroomKit

final class ShortcutEncodingTests: XCTestCase {
    private let keyU: UInt16 = 0x20

    func testModifiersAndLetterBecomeGtkAccelerator() {
        XCTAssertEqual(
            ShortcutEncoding.capture(modifiers: [.option, .command], keyCode: keyU, characters: "u"),
            .accelerator("<Alt><Super>u"))
        XCTAssertEqual(
            ShortcutEncoding.capture(modifiers: [.command, .shift, .control, .option], keyCode: keyU, characters: "U"),
            .accelerator("<Control><Alt><Shift><Super>u"))
    }

    func testEveryAcceleratorParsesBackWithTheHotKeyParser() throws {
        let cases: [(ShortcutModifiers, UInt16, String?)] = [
            ([.control], 0x12, "1"), ([.command, .shift], 0x2B, "<"), ([.option], 0x31, " "),
            ([.command], 0x7B, nil), ([.control], 0x33, nil), ([], 0x60, nil), ([.command], 0x5A, nil),
        ]
        for (modifiers, keyCode, characters) in cases {
            guard
                case .accelerator(let accelerator) = ShortcutEncoding.capture(
                    modifiers: modifiers, keyCode: keyCode, characters: characters)
            else { return XCTFail("no accelerator for \(keyCode)") }
            let chord = try XCTUnwrap(HotKeyChord.parse(accelerator), accelerator)
            XCTAssertEqual(chord.carbonKeyCode, UInt32(keyCode), accelerator)
        }
    }

    func testLettersFollowTheTypedCharacterAndFallBackToThePhysicalKey() {
        XCTAssertEqual(ShortcutEncoding.keyName(keyCode: 0x03, characters: "u"), "u")
        XCTAssertEqual(ShortcutEncoding.keyName(keyCode: keyU, characters: "г"), "u")
        XCTAssertEqual(ShortcutEncoding.keyName(keyCode: 0x12, characters: "!"), "1")
        XCTAssertEqual(ShortcutEncoding.keyName(keyCode: 0x2F, characters: "."), "period")
        XCTAssertEqual(ShortcutEncoding.keyName(keyCode: 0x24, characters: "\r"), "Return")
        XCTAssertNil(ShortcutEncoding.keyName(keyCode: 0x3F, characters: nil))
    }

    func testEscapeCancelsAndDeleteClearsWithoutModifiers() {
        XCTAssertEqual(ShortcutEncoding.capture(modifiers: [], keyCode: 0x35, characters: nil), .cancel)
        XCTAssertEqual(ShortcutEncoding.capture(modifiers: [], keyCode: 0x33, characters: nil), .clear)
        XCTAssertEqual(ShortcutEncoding.capture(modifiers: [], keyCode: 0x75, characters: nil), .clear)
        XCTAssertEqual(
            ShortcutEncoding.capture(modifiers: [.command], keyCode: 0x35, characters: nil),
            .accelerator("<Super>Escape"))
    }

    func testPlainKeysNeedAModifierExceptFunctionKeys() {
        XCTAssertEqual(ShortcutEncoding.capture(modifiers: [], keyCode: keyU, characters: "u"), .needsModifier)
        XCTAssertEqual(ShortcutEncoding.capture(modifiers: [.shift], keyCode: keyU, characters: "U"), .needsModifier)
        XCTAssertEqual(ShortcutEncoding.capture(modifiers: [], keyCode: 0x60, characters: nil), .accelerator("F5"))
        XCTAssertEqual(ShortcutEncoding.capture(modifiers: [.command], keyCode: 0x3F, characters: nil), .unsupported)
    }

    func testSymbolsForDisplay() {
        XCTAssertEqual(ShortcutEncoding.symbols("<Alt><Super>u"), "⌥⌘U")
        XCTAssertEqual(ShortcutEncoding.symbols("<Super><Control>space"), "⌃⌘Space")
        XCTAssertEqual(ShortcutEncoding.symbols("<shift><primary>period"), "⇧⌘.")
        XCTAssertEqual(ShortcutEncoding.symbols("<Ctrl>Page_Up"), "⌃⇞")
        XCTAssertEqual(ShortcutEncoding.symbols("F12"), "F12")
        XCTAssertNil(ShortcutEncoding.symbols("<Hyper>u"))
        XCTAssertNil(ShortcutEncoding.symbols("<Super>"))
        XCTAssertNil(ShortcutEncoding.symbols("<Super>u-"))
    }
}
