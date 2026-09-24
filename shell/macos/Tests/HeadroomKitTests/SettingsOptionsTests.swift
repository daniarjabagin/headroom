import Foundation
import XCTest

@testable import HeadroomKit

final class SettingsOptionsTests: XCTestCase {
    private func accounts() throws -> [Account] {
        try Fixture.decode(DaemonState.self, "state_full").accounts
    }

    func testRefreshIntervalsKeepACustomValue() {
        XCTAssertEqual(SettingsOptions.refreshIntervals(current: 300), SettingsOptions.refreshPresets)
        XCTAssertEqual(SettingsOptions.refreshIntervals(current: 90), [60, 90, 120, 300, 600, 900, 1800, 3600])
    }

    func testHeadlineOptionsListVisibleAccountWindows() throws {
        let options = SettingsOptions.headlines(
            accounts: try accounts(), current: .auto, formatter: DisplayFormatter(language: .en))
        XCTAssertEqual(
            options.map(\.label),
            ["Auto — most critical", "Codex · Work — Session", "Codex · Work — Weekly", "Claude — Session"])
        XCTAssertEqual(options[3].setting, .pinned(accountID: "claude:main", window: "session"))
    }

    func testUnavailablePinIsKept() throws {
        let gone = HeadlineSetting.pinned(accountID: "codex:hidden", window: "session")
        let options = SettingsOptions.headlines(
            accounts: try accounts(), current: gone, formatter: DisplayFormatter(language: .ru))
        XCTAssertEqual(options.first?.label, "Самый критичный")
        XCTAssertEqual(options.last, HeadlineOption(setting: gone, label: "Закреплённый лимит (сейчас недоступен)"))
    }

    func testAccountTexts() throws {
        let all = try accounts()
        let strings = UIStrings(language: .en)
        XCTAssertEqual(SettingsOptions.accountSubtitle(all[0], strings: strings), "Codex · Pro · ada@example.com")
        XCTAssertEqual(SettingsOptions.accountSubtitle(all[1], strings: strings), "Claude · Pro")
        XCTAssertEqual(
            SettingsOptions.removalBody(all[1], strings: strings),
            "Headroom will stop showing this account. The Claude CLI stays signed in; you can sign in again through Headroom."
        )
        XCTAssertEqual(
            SettingsOptions.removalDetail(all[1], strings: strings),
            "Stops showing this account. Its CLI stays signed in.")
    }

    func testHeadroomOwnedAccountTexts() throws {
        let json = try Fixture.text("state_full").replacingOccurrences(
            of: #""owner": "cli""#, with: #""owner": "headroom""#)
        let account = try XCTUnwrap(try Fixture.decode(DaemonState.self, json: json).accounts.first)
        let strings = UIStrings(language: .en)
        XCTAssertEqual(
            SettingsOptions.accountSubtitle(account, strings: strings),
            "Codex · Pro · ada@example.com · added in Headroom")
        XCTAssertEqual(
            SettingsOptions.removalBody(account, strings: strings),
            "Headroom deletes the sign-in it created for this account. The account itself is not affected.")
    }

    func testReorderedFollowsListMoveSemantics() {
        let ids = ["a", "b", "c", "d"]
        XCTAssertEqual(SettingsOptions.reordered(ids, moving: [0], to: 3), ["b", "c", "a", "d"])
        XCTAssertEqual(SettingsOptions.reordered(ids, moving: [3], to: 0), ["d", "a", "b", "c"])
        XCTAssertEqual(SettingsOptions.reordered(ids, moving: [1, 2], to: 4), ["a", "d", "b", "c"])
        XCTAssertEqual(SettingsOptions.reordered(ids, moving: [9], to: 0), ids)
        XCTAssertEqual(SettingsOptions.reordered(ids, moving: [0], to: 7), ids)
    }

    func testProviderMethods() throws {
        let providers = try Fixture.decode(ProvidersPayload.self, "providers").providers
        let strings = UIStrings(language: .en)
        let kimi = try XCTUnwrap(providers.first { $0.id == "kimi" })
        XCTAssertEqual(kimi.supportedMethods.map { $0.summary(strings) }, ["API key", "Sign in with kimi"])
        let cursor = try XCTUnwrap(providers.first { $0.id == "cursor" })
        XCTAssertEqual(cursor.supportedMethods.map { $0.summary(strings) }, ["Detected automatically"])
        XCTAssertEqual(AddAccountMethod.cliLogin(program: "").summary(UIStrings(language: .ru)), "Вход")
    }

    func testUnsupportedMethodsAreDropped() throws {
        let json = #"""
            {"version": 1, "providers": [
              {"id": "a", "display_name": "A", "add_account": [{"kind": "magic"}], "multi_account": false, "local_usage": false},
              {"id": "b", "display_name": "B", "add_account": [{"kind": "magic"}, {"kind": "auto_detect", "reason": "r"}],
               "multi_account": false, "local_usage": false}
            ]}
            """#
        let payload = try Fixture.decode(ProvidersPayload.self, json: json)
        guard case .loaded(let providers) = SettingsStore.list(payload) else { return XCTFail("expected providers") }
        XCTAssertEqual(providers.map(\.id), ["b"])
        XCTAssertEqual(providers.first?.supportedMethods, [.autoDetect(reason: "r")])
    }

    func testUnknownProvidersVersionFails() throws {
        let payload = try Fixture.decode(ProvidersPayload.self, json: #"{"version": 2, "providers": []}"#)
        XCTAssertEqual(SettingsStore.list(payload), .failed("The Headroom service lists providers in version 2"))
    }
}
