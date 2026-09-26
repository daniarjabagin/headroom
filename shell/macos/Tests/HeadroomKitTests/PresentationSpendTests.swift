import Foundation
import XCTest

@testable import HeadroomKit

final class PresentationSpendTests: XCTestCase {
    private static func selection(_ period: SpendPeriodPreference) -> SpendSelection {
        SpendSelection(period: period, unit: .cost, breakdown: .models)
    }

    func testLegendKeepsDaemonOrderAndRingFractions() throws {
        let card = SpendCardModel.make(
            spend: try Build.full().spend, selection: Self.selection(.today), formatter: Build.english)
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
        let card = SpendCardModel.make(
            spend: try Build.full().spend, selection: Self.selection(.yesterday), formatter: Build.english)
        XCTAssertEqual(card.fractions, [1])
        XCTAssertEqual(card.entries.first?.tokensLine, "615 tokens")
        XCTAssertEqual(card.info, "Estimated from local logs and public pricing. Some models have no public price yet.")
        guard case .models(let popover) = card.entries.first?.tip else { return XCTFail("expected popover") }
        XCTAssertEqual(popover.title, "Codex · Yesterday")
        XCTAssertEqual(popover.rows.map(\.name), ["gpt-5.5", "unknown"])
        XCTAssertEqual(popover.rows.map(\.cost), ["$0.00", "unpriced"])
        XCTAssertEqual(popover.total, "$0.00")
        XCTAssertEqual(
            popover.footnotes,
            ["Estimated from local logs and public pricing.", "Some models have no public price yet."])
    }

    func testEmptyPeriodAndVisibility() throws {
        let empty = try Fixture.decode(DaemonState.self, "state_empty")
        let card = SpendCardModel.make(
            spend: empty.spend, selection: Self.selection(.last30Days), formatter: Build.english)
        XCTAssertTrue(card.isEmpty)
        XCTAssertFalse(SpendCardModel.shows(empty))
        XCTAssertTrue(SpendCardModel.shows(try Build.full()))
    }

    func testOtherModelsRowComesLast() {
        let breakdown = ModelBreakdown.make(
            title: "Today · Claude",
            models: [
                ModelUsage(
                    model: "opus", totalTokens: 2_500_000, costUSDMicros: 12_340_000, partial: false,
                    costPerMTokUSDMicros: 4_936_000)
            ],
            other: OtherModels(
                count: 3, totalTokens: 1500, costUSDMicros: 20_000, partial: false, costPerMTokUSDMicros: nil),
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

    func testEveryKnownProviderHasItsOwnSeriesColor() throws {
        let ids = try Fixture.decode(ProvidersPayload.self, "providers").providers.map(\.id)
        XCTAssertEqual(ids.count, 19)
        let colors = ids.map(ProviderStyle.seriesColor(for:))
        XCTAssertEqual(Set(colors.map(\.light)).count, ids.count)
        XCTAssertEqual(Set(colors.map(\.dark)).count, ids.count)
        XCTAssertEqual(ProviderStyle.seriesColor(for: "kilo"), SeriesColor(light: 0xB59A00, dark: 0xF8F675))
        XCTAssertEqual(ProviderStyle.seriesColor(for: "moonshot"), SeriesColor(light: 0x475A78, dark: 0xA5B4CC))
    }
}
