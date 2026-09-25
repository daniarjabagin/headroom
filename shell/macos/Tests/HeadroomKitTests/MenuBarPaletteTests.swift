import XCTest

@testable import HeadroomKit

final class MenuBarPaletteTests: XCTestCase {
    func testOnlyWarningAndCriticalLeaveTheTemplate() {
        XCTAssertEqual(PanelAppearance.resolve(tone: .good, darkPanel: true), .template)
        XCTAssertEqual(PanelAppearance.resolve(tone: .neutral, darkPanel: false), .template)
        XCTAssertEqual(PanelAppearance.resolve(tone: nil, darkPanel: true), .template)
        XCTAssertEqual(PanelAppearance.resolve(tone: .warning, darkPanel: true), .dark)
        XCTAssertEqual(PanelAppearance.resolve(tone: .critical, darkPanel: false), .light)
    }

    func testToneColorsFollowThePanelTable() {
        let cases: [(Tone, PanelAppearance, UInt32, UInt32)] = [
            (.warning, .dark, 0xFFD60A, 0xFFD60A),
            (.warning, .light, 0xB76B00, 0xFFCC00),
            (.critical, .dark, 0xFF453A, 0xFF453A),
            (.critical, .light, 0xE0281E, 0xFF3B30),
        ]
        for (tone, appearance, text, graphic) in cases {
            let colors = MenuBarPalette.colors(tone: tone, appearance: appearance)
            XCTAssertEqual(colors.text, PanelColor(hex: text), "\(tone) \(appearance)")
            XCTAssertEqual(colors.graphic, PanelColor(hex: graphic), "\(tone) \(appearance)")
        }
    }

    func testGoodStaysInThePanelForeground() {
        let dark = MenuBarPalette.colors(tone: .good, appearance: .dark)
        XCTAssertEqual(dark.text, PanelColor(hex: 0xFFFFFF))
        XCTAssertEqual(dark.graphic, PanelColor(hex: 0xFFFFFF))
        let template = MenuBarPalette.colors(tone: .good, appearance: .template)
        XCTAssertEqual(template.text, PanelColor(hex: 0x000000))
    }

    func testTrackTickAndLetterAreForegroundAlphas() {
        let light = MenuBarPalette.colors(tone: .warning, appearance: .light)
        XCTAssertEqual(light.foreground, PanelColor(hex: 0x000000, alpha: 0.85))
        XCTAssertEqual(light.track.alpha, 0.85 * 0.28, accuracy: 1e-9)
        XCTAssertEqual(light.tick.alpha, 0.85 * 0.8, accuracy: 1e-9)
        XCTAssertEqual(light.secondary.alpha, 0.85 * 0.67, accuracy: 1e-9)
        XCTAssertEqual(light.track.red, 0)
    }

    func testBarFillKeepsAFullCircleForAnyNonZeroValue() {
        XCTAssertEqual(MenuBarBar.fillWidth(0), 0)
        XCTAssertEqual(MenuBarBar.fillWidth(0.01), 5)
        XCTAssertEqual(MenuBarBar.fillWidth(0.5), 13)
        XCTAssertEqual(MenuBarBar.fillWidth(1), 26)
        XCTAssertEqual(MenuBarBar.fillWidth(1.5), 26)
        XCTAssertEqual(MenuBarBar.fillWidth(.nan), 0)
    }

    func testTickStaysInsideTheBar() {
        XCTAssertEqual(MenuBarBar.tickCenter(0.5), 13)
        XCTAssertEqual(MenuBarBar.tickCenter(0), 1)
        XCTAssertEqual(MenuBarBar.tickCenter(1), 25)
        XCTAssertEqual(MenuBarBar.tickCenter(.infinity), 1)
    }
}
