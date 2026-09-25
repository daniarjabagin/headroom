import Foundation
import XCTest

@testable import HeadroomKit

final class PresentationShareTests: XCTestCase {
    private func card(_ state: DaemonState, index: Int = 0) throws -> ShareCardModel {
        let sections = AccountSectionModel.sections(state, formatter: Build.english)
        return try XCTUnwrap(
            ShareCardModel.make(sections[index], state: state, now: try Build.now(), formatter: Build.english))
    }

    func testCombinedCardShowsPooledHeroWithoutPersonalData() throws {
        let state = try Fixture.decode(DaemonState.self, "state_combined")
        let model = try card(state)
        XCTAssertEqual(model.provider, "codex")
        XCTAssertEqual(model.providerName, "Codex")
        XCTAssertEqual(model.stamp, "CODEX LIMITS · 23 SEP 2026")
        XCTAssertEqual(model.fileName, "headroom-codex-2026-09-23-1000.png")
        XCTAssertEqual(model.heroWord, "left")
        XCTAssertEqual(model.tagline, "Know what's left.")
        XCTAssertGreaterThan(model.heroMeters.count, 1)
        XCTAssertTrue(model.heroDetail.hasPrefix("of "))
        let everything =
            ([model.subtitle ?? "", model.stamp, model.hero, model.heroDetail, model.text]
            + model.rows.flatMap { [$0.label, $0.headline, $0.trailing] }).joined(separator: "\n")
        for account in state.accounts {
            for personal in [account.email, account.label, account.id].compactMap({ $0 }) {
                XCTAssertFalse(everything.contains(personal), personal)
            }
        }
    }

    func testSingleAccountCardUsesTheHeadlineWindow() throws {
        let state = try Build.mutated("state_full") { object in
            object["headline"] = [
                "account_id": "codex:work", "provider": "codex", "provider_name": "Codex", "account_label": "Work",
                "window": "session", "window_label": "Session", "used_percent": 60.0, "remaining_percent": 40.0,
                "tone": "good",
            ]
        }
        let model = try card(state)
        let row = try XCTUnwrap(model.rows.first { $0.id == "session" })
        XCTAssertEqual(model.hero, Build.english.panelPercent(row.percent))
        XCTAssertEqual(model.heroMeters.count, 1)
        XCTAssertTrue(model.heroDetail.hasPrefix(row.label))
        XCTAssertEqual(model.subtitle, "· Pro")
        XCTAssertTrue(model.text.hasPrefix("Codex · Pro\n"))
        XCTAssertEqual(model.text.split(separator: "\n").count, model.rows.count + 1)
        XCTAssertFalse(model.text.contains("Work"))
    }

    func testSubtitleNamesAccountCountAndPlans() throws {
        let accounts = try [
            Build.accountJSON(id: "a", plan: "\"Pro\""), Build.accountJSON(id: "b", plan: "\"Plus\""),
            Build.accountJSON(id: "c", plan: "\"Pro\""),
        ].map { try Fixture.decode(Account.self, json: $0) }
        XCTAssertEqual(ShareCardModel.subtitle(accounts, strings: Build.english.strings), "· 3 accounts · Pro + Plus")
        XCTAssertEqual(ShareCardModel.subtitle(Array(accounts.prefix(1)), strings: Build.english.strings), "· Pro")
        XCTAssertNil(ShareCardModel.subtitle([], strings: Build.english.strings))
    }

    func testFileNameIsSafeAndDated() throws {
        let date = try Fixture.timestamp("2026-01-05T07:04:00Z").date
        XCTAssertEqual(
            ShareDate.fileName(provider: "../x", date: date, formatter: Build.english),
            "headroom-provider-2026-01-05-0704.png")
        XCTAssertEqual(ShareDate.day(date, Build.russian), "5 янв. 2026")
    }

    func testStyleFollowsTheBrandTokens() {
        XCTAssertEqual(ShareCardStyle.make(dark: false).background, 0xF0F0ED)
        XCTAssertEqual(ShareCardStyle.make(dark: true).background, 0x151617)
        XCTAssertEqual(ShareCardStyle.paper.tone(.critical), 0xFF3B30)
        XCTAssertEqual(ShareCardStyle.logicalWidth * ShareCardStyle.scale, 1200)
        XCTAssertEqual(ShareCardStyle.logicalHeight * ShareCardStyle.scale, 630)
    }
}
