import Foundation
import XCTest

@testable import HeadroomKit

final class SettingsChangeTests: XCTestCase {
    private func settings() throws -> Settings {
        try Fixture.decode(
            Settings.self,
            json: #"""
                {
                  "refresh_interval_secs": 300,
                  "notifications": { "almost_out": true, "cutting_it_close": true, "will_run_out": true, "reset": false },
                  "headline": { "mode": "auto" },
                  "reduced_motion": false,
                  "display": {
                    "theme": "system", "language": "system", "value_mode": "left", "reset_format": "countdown",
                    "panel_label": "percent", "show_spend": true, "show_account_spend": true, "show_trend": true,
                    "show_forecast": true, "translucent": false, "hidden_windows": { "codex:a": ["weekly"] }
                  }
                }
                """#)
    }

    private func patchText(_ change: SettingsChange) throws -> String {
        try RPCCodec.encodeString(change.patch)
    }

    func testDisplayPatchesAreMergePatches() throws {
        XCTAssertEqual(try patchText(.theme(.dark)), #"{"display":{"theme":"dark"}}"#)
        XCTAssertEqual(try patchText(.language(.ru)), #"{"display":{"language":"ru"}}"#)
        XCTAssertEqual(try patchText(.valueMode(.used)), #"{"display":{"value_mode":"used"}}"#)
        XCTAssertEqual(try patchText(.resetFormat(.exact)), #"{"display":{"reset_format":"exact"}}"#)
        XCTAssertEqual(try patchText(.panelLabel(.window)), #"{"display":{"panel_label":"window"}}"#)
        XCTAssertEqual(try patchText(.translucent(true)), #"{"display":{"translucent":true}}"#)
        XCTAssertEqual(try patchText(.section(.showTrend, false)), #"{"display":{"show_trend":false}}"#)
    }

    func testTopLevelPatches() throws {
        XCTAssertEqual(try patchText(.reducedMotion(true)), #"{"reduced_motion":true}"#)
        XCTAssertEqual(try patchText(.refreshInterval(600)), #"{"refresh_interval_secs":600}"#)
        XCTAssertEqual(try patchText(.refreshInterval(5)), #"{"refresh_interval_secs":60}"#)
        XCTAssertEqual(try patchText(.refreshInterval(99_999)), #"{"refresh_interval_secs":3600}"#)
        XCTAssertEqual(try patchText(.notification(.reset, true)), #"{"notifications":{"reset":true}}"#)
        XCTAssertEqual(try patchText(.headline(.auto)), #"{"headline":{"mode":"auto"}}"#)
        XCTAssertEqual(
            try patchText(.headline(.pinned(accountID: "codex:a", window: "session"))),
            #"{"headline":{"account_id":"codex:a","mode":"pinned","window":"session"}}"#)
    }

    func testHiddenWindowsPatchReplacesOneAccountOrRemovesIt() throws {
        XCTAssertEqual(
            try patchText(.hiddenWindows(accountID: "codex:a", windows: ["weekly", "session", "weekly", ""])),
            #"{"display":{"hidden_windows":{"codex:a":["weekly","session"]}}}"#)
        XCTAssertEqual(
            try patchText(.hiddenWindows(accountID: "codex:a", windows: [])),
            #"{"display":{"hidden_windows":{"codex:a":null}}}"#)
    }

    func testAppliedMatchesPatch() throws {
        let base = try settings()
        XCTAssertEqual(SettingsChange.theme(.light).applied(to: base).display.theme, .light)
        XCTAssertFalse(SettingsChange.section(.showSpend, false).applied(to: base).display.showSpend)
        XCTAssertEqual(SettingsChange.refreshInterval(10).applied(to: base).refreshIntervalSecs, 60)
        XCTAssertTrue(SettingsChange.notification(.reset, true).applied(to: base).notifications.reset)
        XCTAssertTrue(SettingsChange.reducedMotion(true).applied(to: base).reducedMotion)
        let pinned = HeadlineSetting.pinned(accountID: "codex:a", window: "weekly")
        XCTAssertEqual(SettingsChange.headline(pinned).applied(to: base).headline, pinned)
        let cleared = SettingsChange.hiddenWindows(accountID: "codex:a", windows: []).applied(to: base)
        XCTAssertEqual(cleared.display.hiddenWindows, [:])
        let hidden = SettingsChange.hiddenWindows(accountID: "claude:b", windows: ["session"]).applied(to: base)
        XCTAssertEqual(hidden.display.hiddenWindows, ["codex:a": ["weekly"], "claude:b": ["session"]])
    }

    func testDisplayQueries() throws {
        let display = try settings().display
        XCTAssertTrue(display.isHidden(accountID: "codex:a", windowID: "weekly"))
        XCTAssertFalse(display.isHidden(accountID: "codex:a", windowID: "session"))
        XCTAssertEqual(
            display.hiddenWindows(after: "session", hidden: true, accountID: "codex:a"), ["weekly", "session"])
        XCTAssertEqual(display.hiddenWindows(after: "weekly", hidden: false, accountID: "codex:a"), [])
        XCTAssertTrue(display.isShown(.showForecast))
        XCTAssertTrue(try settings().notifications.isEnabled(.almostOut))
        XCTAssertFalse(try settings().notifications.isEnabled(.reset))
    }
}
