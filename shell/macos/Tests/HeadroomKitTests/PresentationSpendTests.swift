import Foundation
import XCTest

@testable import HeadroomKit

final class PresentationSpendTests: XCTestCase {
    func testLegendKeepsDaemonOrderAndRingFractions() throws {
        let card = SpendCardModel.make(spend: try Build.full().spend, period: .today, formatter: Build.english)
        XCTAssertEqual(card.entries.map(\.name), ["Claude", "Codex"])
        XCTAssertEqual(card.entries.map(\.amount), ["$0.01", "$0.00"])
        XCTAssertEqual(card.centerAmount, "$0.01")
        XCTAssertEqual(card.fractions.count, 2)
        XCTAssertEqual(card.fractions.reduce(0, +), 1, accuracy: 1e-9)
        XCTAssertEqual(card.fractions[0], 10_000.0 / 12_400, accuracy: 1e-9)
        XCTAssertNil(card.entries[0].tokensLine)
        XCTAssertEqual(card.entries[0].color, SeriesColor(light: 0xD97757, dark: 0xD97757))
        XCTAssertEqual(card.info, "Estimated from local logs and public pricing.")
    }

    func testSingleProviderAddsExactTokensAndPartialBreakdown() throws {
        let card = SpendCardModel.make(spend: try Build.full().spend, period: .yesterday, formatter: Build.english)
        XCTAssertEqual(card.fractions, [1])
        XCTAssertEqual(card.entries.first?.tokensLine, "615 tokens")
        XCTAssertEqual(card.info, "Estimated from local logs and public pricing. Some models have no public price yet.")
        guard case .breakdown(let breakdown) = card.entries.first?.tip else { return XCTFail("expected breakdown") }
        XCTAssertEqual(breakdown.title, "Yesterday · Codex")
        XCTAssertEqual(
            breakdown.rows,
            [
                ModelBreakdownRow(name: "gpt-5.5", tokens: "600", cost: "$0.00"),
                ModelBreakdownRow(name: "unknown *", tokens: "15", cost: "unpriced"),
            ])
        XCTAssertEqual(breakdown.total, "$0.00 · 615 tokens")
        XCTAssertEqual(breakdown.partialNote, "* Partly unpriced, cost leaves it out")
    }

    func testEmptyPeriodAndVisibility() throws {
        let empty = try Fixture.decode(DaemonState.self, "state_empty")
        let card = SpendCardModel.make(spend: empty.spend, period: .last30Days, formatter: Build.english)
        XCTAssertTrue(card.isEmpty)
        XCTAssertFalse(SpendCardModel.shows(empty))
        XCTAssertTrue(SpendCardModel.shows(try Build.full()))
        XCTAssertEqual(SpendPeriod.allCases.map { $0.title(Build.russian.strings) }, ["Сегодня", "Вчера", "30 дней"])
    }

    func testOtherModelsRowComesLast() {
        let breakdown = ModelBreakdown.make(
            title: "Today · Claude",
            models: [ModelUsage(model: "opus", totalTokens: 2_500_000, costUSDMicros: 12_340_000, partial: false)],
            other: OtherModels(count: 3, totalTokens: 1500, costUSDMicros: 20_000, partial: false),
            costMicros: 12_360_000, totalTokens: 2_501_500, formatter: Build.english)
        XCTAssertEqual(breakdown?.rows.map(\.name), ["opus", "Other (3)"])
        XCTAssertEqual(breakdown?.rows.map(\.tokens), ["2.5M", "1.5K"])
        XCTAssertEqual(breakdown?.total, "$12.36 · 2,501,500 tokens")
        XCTAssertNil(breakdown?.partialNote)
        XCTAssertNil(
            ModelBreakdown.make(
                title: "", models: [], other: nil, costMicros: 0, totalTokens: 0, formatter: Build.english))
    }

    func testRingAmounts() {
        let cases: [(Int64, String)] = [
            (0, "$0.00"), (18_420_000, "$18.42"), (99_994_999, "$99.99"), (100_000_000, "$100"),
            (463_490_000, "$463"), (9_999_990_000, "$10000"), (10_000_000_000, "$10K"), (2_060_000_000, "$2060"),
        ]
        for (micros, expected) in cases {
            XCTAssertEqual(Build.english.ringUSD(micros: micros), expected, "\(micros)")
        }
    }

    func testSpendLines() {
        XCTAssertEqual(Build.english.spendLine(costMicros: 0, totalTokens: 0), "No data")
        XCTAssertEqual(Build.english.spendLine(costMicros: 4_080_000, totalTokens: 1_200_000), "$4.08 · 1.2M tokens")
        XCTAssertEqual(
            Build.english.exactSpendLine(costMicros: 4_080_000, totalTokens: 1_200_000, partial: true),
            "$4.08 · 1,200,000 tokens · some models unpriced")
    }

    func testSeriesColorsFallBackByStableHash() {
        XCTAssertEqual(ProviderStyle.seriesColor(for: "codex"), SeriesColor(light: 0x10A37F, dark: 0x10A37F))
        XCTAssertEqual(ProviderStyle.stableHash("ab"), 97 * 31 + 98)
        XCTAssertEqual(ProviderStyle.seriesColor(for: "ab"), SeriesColor(light: 0x5856D6, dark: 0x5E5CE6))
        XCTAssertEqual(ProviderStyle.iconResource(for: "claude"), "claude")
        XCTAssertNil(ProviderStyle.iconResource(for: "../etc"))
        XCTAssertNil(ProviderStyle.iconResource(for: ""))
    }
}
