import Foundation
import XCTest

@testable import HeadroomKit

final class PresentationUsageTests: XCTestCase {
    func testTrendBarsScaleToThePeakWithStubsAndMinimumHeight() throws {
        let state = try Build.full()
        let codex = try XCTUnwrap(state.usage.first { $0.provider == "codex" })
        let bars = UsageRows.trendBars(codex.daily, formatter: Build.english)
        XCTAssertEqual(bars.count, 30)
        XCTAssertEqual(bars.last?.height, 18)
        XCTAssertEqual(bars[28].height, 9)
        XCTAssertEqual(bars[0].height, 2)
        XCTAssertEqual(bars.last?.tip, .day(title: "Wed, Sep 23", detail: "1.2K tokens · $0.00"))
        XCTAssertEqual(bars[0].tip, .day(title: "Tue, Aug 25", detail: "No usage"))
        XCTAssertEqual(UsageRows.barHeight(1, peak: 1_000), 3)
    }

    func testShortHistoryIsPaddedAtTheStartWithoutTips() throws {
        let day = try Fixture.decode(
            DailyUsage.self, json: #"{"date":"2026-09-23","total_tokens":10,"cost_usd_micros":0,"partial":true}"#)
        let bars = UsageRows.trendBars([day], formatter: Build.russian)
        XCTAssertEqual(bars.count, 30)
        XCTAssertNil(bars[0].tip)
        XCTAssertEqual(bars[29].tip, .day(title: "ср, 23 сент.", detail: "10 токенов · $0.00 · часть моделей без цены"))
    }

    func testSpendRowsFindTheAccountsUsageHome() throws {
        let state = try Build.full()
        let codex = try XCTUnwrap(state.accounts.first)
        let usage = try XCTUnwrap(UsageRows.usage(for: codex, in: state.usage))
        let rows = UsageRows.spendRows(usage, providerName: "Codex", formatter: Build.english)
        XCTAssertEqual(rows.map(\.value), ["$0.00 · 1.2K tokens", "$0.00 · 615 tokens", "$0.00 · 1.8K tokens"])
        guard case .breakdown(let breakdown) = rows[0].tip else { return XCTFail("expected breakdown") }
        XCTAssertEqual(breakdown.title, "Today · Codex")
        let hidden = try XCTUnwrap(state.accounts.last)
        XCTAssertNil(UsageRows.usage(for: hidden, in: state.usage))
    }

    func testBalanceValues() throws {
        let balances = try Fixture.decode(
            [Balance].self,
            json: """
                [{"id":"credits","label":"Credits","kind":"usd","usd_micros":2500000},
                {"id":"extra_usage","label":"Extra usage","kind":"usd"},
                {"id":"msgs","label":"Messages","kind":"count","value":-3},
                {"id":"odd","label":"Odd","kind":"points"}]
                """)
        let rows = UsageRows.balanceRows(balances, formatter: Build.english)
        XCTAssertEqual(rows.map(\.title), ["Credits", "Extra usage", "Messages", "Odd"])
        XCTAssertEqual(rows.map(\.value), ["$2.50", "No data", "-3", "No data"])
    }

    func testBalanceTitlesAreTranslatedByLabel() throws {
        let balances = try Fixture.decode(
            [Balance].self,
            json: """
                [{"id":"credits","label":"Organization credits","kind":"usd","usd_micros":2500000},
                {"id":"extra_usage","label":"Extra usage","kind":"usd"},
                {"id":"cash","label":"Cash","kind":"money","currency":"CNY","micros":0},
                {"id":"msgs","label":"Messages","kind":"count","value":3}]
                """)
        let rows = UsageRows.balanceRows(balances, formatter: Build.russian)
        XCTAssertEqual(
            rows.map(\.title), ["Кредиты организации", "Доп. использование", "Денежный баланс", "Messages"])
    }

    func testMoneyBalancesKeepTheirCurrency() throws {
        let balances = try Fixture.decode([Balance].self, "balances")
        let rows = UsageRows.balanceRows(balances, formatter: Build.english)
        XCTAssertEqual(
            rows.map(\.title),
            ["Credits", "Balance", "Granted", "Balance (USD)", "Balance (GBP)", "Pending", "Requests", "Points"])
        XCTAssertEqual(
            rows.map(\.value),
            ["$12.50", "¥12.50", "-¥3.00", "$1,234.57", "12.50 GBP", "No data", "1,500 requests", "No data"])
    }

    func testAccountCardRendersMoneyBalanceRows() throws {
        let balances = String(decoding: try Fixture.data("balances"), as: UTF8.self)
        let state = try Build.state(accounts: [Build.accountJSON(id: "a", balances: balances)])
        let section = try XCTUnwrap(AccountSectionModel.sections(state, formatter: Build.english).first)
        guard case .limits(let limits) = section.body else { return XCTFail("expected limits") }
        XCTAssertEqual(limits.extras.first { $0.id == "balance:balance_cny" }?.value, "¥12.50")
        XCTAssertEqual(limits.extras.first { $0.id == "balance:granted_cny" }?.value, "-¥3.00")
    }

    func testDayTitleRejectsMalformedDates() {
        XCTAssertEqual(Build.english.dayTitle("2026-02-30"), "2026-02-30")
        XCTAssertEqual(Build.english.dayTitle("soon"), "soon")
        XCTAssertEqual(Build.english.dayTitle("2026-10-04"), "Sun, Oct 4")
    }
}
