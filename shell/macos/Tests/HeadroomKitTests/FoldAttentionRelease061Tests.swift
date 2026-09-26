import Foundation
import XCTest

@testable import HeadroomKit

final class FoldAttentionRelease061Tests: XCTestCase {
    private let http = #"{"kind":"http","message":"HTTP 500"}"#
    private let network = #"{"kind":"network","message":"offline"}"#

    private func attention(_ accounts: [String], offline: Bool = false) throws -> FoldAttention {
        let state = try Build.state(accounts: accounts, offline: offline)
        return FoldAttention.make(AccountSectionModel.sections(state, formatter: Build.english), state: state)
    }

    private func account(
        _ id: String, status: String = "fresh", error: String = "null", tones: [String] = [],
        hidden: [Bool] = []
    ) -> String {
        let windows = tones.enumerated().map { index, tone in
            Build.windowJSON(id: "w\(index)", tone: tone, hidden: hidden.indices.contains(index) && hidden[index])
        }
        return Build.accountJSON(id: id, status: status, error: error, windows: windows)
    }

    func testCalmFoldHasNoMark() throws {
        XCTAssertEqual(FoldAttention.make([], state: try Build.state(accounts: [])), .calm)
        XCTAssertEqual(try attention([account("a", tones: ["good", "neutral"])]), .calm)
        XCTAssertEqual(try attention([account("a", tones: ["critical", "good"], hidden: [true])]), .calm)
    }

    func testWorstWindowToneMarksTheFold() throws {
        XCTAssertEqual(
            try attention([account("a", tones: ["warning"]), account("b", tones: ["good"])]),
            FoldAttention(kind: .tone, tone: .warning, count: 1))
        XCTAssertEqual(
            try attention([account("a", tones: ["warning"]), account("b", tones: ["critical"])]),
            FoldAttention(kind: .tone, tone: .critical, count: 2))
    }

    func testNoticesOutrankTones() throws {
        XCTAssertEqual(
            try attention([account("a", status: "signed_out")]), FoldAttention(kind: .notice, tone: .warning, count: 1))
        XCTAssertEqual(try attention([account("a", status: "no_subscription")]).tone, .warning)
        XCTAssertEqual(
            try attention([
                account("a", status: "signed_out"), account("b", status: "error", error: http),
                account("c", tones: ["warning"]),
            ]),
            FoldAttention(kind: .notice, tone: .critical, count: 3))
        XCTAssertEqual(try attention([account("a", status: "refreshing", error: http)]).tone, .critical)
    }

    func testOfflineNetworkFailureMatchesTheCard() throws {
        XCTAssertEqual(try attention([account("a", status: "error", error: network)], offline: true), .calm)
        XCTAssertEqual(try attention([account("a", status: "error", error: network)]).tone, .critical)
    }

    func testCombinedGroupUsesGroupWindowsAndMemberStatuses() throws {
        let state = try Build.combined { _ in }
        let sections = AccountSectionModel.sections(state, formatter: Build.english)
        let group = sections.filter { if case .combined = $0.body { true } else { false } }
        XCTAssertEqual(group.count, 1)
        XCTAssertEqual(FoldAttention.make(group, state: state), .calm)
        let signedOut = sections.filter { $0.provider == "claude" }
        XCTAssertEqual(
            FoldAttention.make(signedOut, state: state), FoldAttention(kind: .notice, tone: .warning, count: 1))
    }

    func testTextsAndSummary() throws {
        let strings = Build.english.strings
        XCTAssertNil(FoldAttention.calm.text(strings))
        XCTAssertEqual(FoldAttention(kind: .tone, tone: .warning, count: 1).text(strings), "1 needs attention")
        XCTAssertEqual(FoldAttention(kind: .notice, tone: .critical, count: 3).text(strings), "3 need attention")
        XCTAssertEqual(
            FoldAttention(kind: .notice, tone: .warning, count: 2).text(Build.russian.strings), "Требуют внимания: 2")
        XCTAssertEqual(
            FoldAttention(kind: .notice, tone: .warning, count: 1).text(Build.russian.strings), "Требуют внимания: 1")
        let state = try Build.state(accounts: [account("grok:a", status: "signed_out")])
        let sections = AccountSectionModel.sections(state, formatter: Build.english)
        let summary = AccountFold.summary(
            sections, attention: FoldAttention.make(sections, state: state), strings: strings)
        XCTAssertEqual(summary.attentionText, "1 needs attention")
        XCTAssertEqual(summary.accessibilityLabel, "1 more · Codex. 1 needs attention")
    }
}
