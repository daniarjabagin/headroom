import Foundation
import XCTest

@testable import HeadroomKit

final class SettingsTests: XCTestCase {
    private let documented = #"""
        {
          "refresh_interval_secs": 300,
          "notifications": { "almost_out": true, "cutting_it_close": true, "will_run_out": true, "reset": false },
          "headline": { "mode": "auto" },
          "reduced_motion": false,
          "display": {
            "theme": "system", "language": "system", "value_mode": "left", "reset_format": "countdown",
            "panel_label": "percent", "show_spend": true, "show_account_spend": true, "show_trend": true,
            "show_forecast": true, "translucent": false, "hidden_windows": {}
          }
        }
        """#

    func testDocumentedSettingsDecode() throws {
        let settings = try Fixture.decode(Settings.self, json: documented)
        XCTAssertEqual(settings.refreshIntervalSecs, 300)
        XCTAssertEqual(settings.headline, .auto)
        XCTAssertFalse(settings.notifications.reset)
        XCTAssertEqual(settings.display.panelLabel, .percent)
    }

    func testSettingsRoundTripWithSnakeCaseKeys() throws {
        var settings = try Fixture.decode(Settings.self, json: documented)
        settings.headline = .pinned(accountID: "codex:1a2b", window: "session")
        settings.display.hiddenWindows = ["codex:1a2b_x": ["weekly"]]
        let encoded = try RPCCodec.encodeString(settings)
        XCTAssertTrue(encoded.contains(#""refresh_interval_secs":300"#))
        XCTAssertTrue(encoded.contains(#""headline":{"account_id":"codex:1a2b","mode":"pinned","window":"session"}"#))
        XCTAssertTrue(encoded.contains(#""hidden_windows":{"codex:1a2b_x":["weekly"]}"#))
        XCTAssertEqual(try Fixture.decode(Settings.self, json: encoded), settings)
    }

    func testPatchKeepsIntegersIntegral() throws {
        let patch: [String: JSONValue] = [
            "refresh_interval_secs": .integer(600), "display": .object(["hidden_windows": .object(["a": .null])]),
        ]
        XCTAssertEqual(
            try RPCCodec.encodeString(patch),
            #"{"display":{"hidden_windows":{"a":null}},"refresh_interval_secs":600}"#)
    }
}
