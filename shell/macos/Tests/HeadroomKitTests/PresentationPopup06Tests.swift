import Foundation
import XCTest

@testable import HeadroomKit

final class PresentationPopup06Tests: XCTestCase {
    private func status(
        indicator: String, tone: String, title: String? = "Elevated errors on Claude API",
        stage: String? = "investigating", url: String = "https://status.claude.com"
    ) throws -> ProviderStatus {
        let titleJSON = title.map { "\"\($0)\"" } ?? "null"
        let stageJSON = stage.map { "\"\($0)\"" } ?? "null"
        return try Fixture.decode(
            ProviderStatus.self,
            json: """
                {"provider":"claude","indicator":"\(indicator)","tone":"\(tone)","title":\(titleJSON),
                 "stage":\(stageJSON),"started_at":"2026-09-23T09:22:00Z","url":"\(url)"}
                """)
    }

    private func notice(_ status: ProviderStatus, _ formatter: DisplayFormatter = Build.english) throws
        -> StatusNoticeModel?
    {
        StatusNoticeModel.make(provider: "claude", statuses: [status], now: try Build.now(), formatter: formatter)
    }

    func testIncidentIsAmberWithStageAndStart() throws {
        let model = try XCTUnwrap(try notice(try status(indicator: "minor", tone: "warning")))
        XCTAssertEqual(model.kind, .warning)
        XCTAssertEqual(model.title, "Incident · Elevated errors on Claude API")
        XCTAssertEqual(model.heading, "Incident")
        XCTAssertEqual(model.detail, "Investigating")
        XCTAssertEqual(model.started, "Started 38m ago")
        XCTAssertEqual(model.startedTip, "Started at 09:22")
        XCTAssertEqual(model.elapsed, "· 38m")
        XCTAssertEqual(model.url?.absoluteString, "https://status.claude.com")
        XCTAssertEqual(model.linkTitle, "Status page")
    }

    func testOutageIsRedAndClearOrMissingShowsNothing() throws {
        let outage = try XCTUnwrap(
            try notice(try status(indicator: "critical", tone: "critical", title: nil, stage: "identified")))
        XCTAssertEqual(outage.kind, .error)
        XCTAssertEqual(outage.title, "Major outage")
        XCTAssertEqual(outage.detail, "Identified — a fix is on the way")
        let degraded = try XCTUnwrap(
            try notice(try status(indicator: "minor", tone: "warning", title: nil, stage: nil)))
        XCTAssertEqual(degraded.title, "Degraded performance")
        XCTAssertNil(degraded.detail)
        XCTAssertNil(try notice(try status(indicator: "none", tone: "neutral")))
        XCTAssertNil(
            StatusNoticeModel.make(
                provider: "codex", statuses: [try status(indicator: "major", tone: "critical")],
                now: try Build.now(), formatter: Build.english))
        let insecure = try XCTUnwrap(try notice(try status(indicator: "major", tone: "critical", url: "http://x.io")))
        XCTAssertNil(insecure.url)
        XCTAssertEqual(insecure.heading, "Partial outage")
        let russian = try XCTUnwrap(
            try notice(try status(indicator: "maintenance", tone: "warning", stage: "in_progress"), Build.russian))
        XCTAssertEqual(russian.heading, "Техработы")
        XCTAssertEqual(russian.detail, "Идут работы")
        XCTAssertEqual(StatusNoticeModel.stage("scheduled_soon", strings: Build.english.strings), "Scheduled soon")
    }

    private func foldState() throws -> DaemonState {
        let accounts = [
            Build.accountJSON(id: "claude:a", provider: "claude", usageHome: "~/.claude"),
            Build.accountJSON(id: "copilot:a", provider: "copilot"),
            Build.accountJSON(id: "grok:a", provider: "grok"),
        ]
        return try Build.mutated("state_empty") { object in
            object["accounts"] = accounts.enumerated().compactMap { index, json -> [String: Any]? in
                guard var account = try? JSONSerialization.jsonObject(with: Data(json.utf8)) as? [String: Any] else {
                    return nil
                }
                account["collapsed"] = index > 0
                return account
            }
        }
    }

    func testFoldSplitsPinnedAndCollapsedSections() throws {
        let state = try foldState()
        let sections = AccountSectionModel.sections(state, formatter: Build.english)
        let fold = AccountFold.make(sections, state: state, strings: Build.english.strings)
        XCTAssertEqual(fold.pinned.map(\.id), ["claude:a"])
        XCTAssertEqual(fold.folded.map(\.id), ["copilot:a", "grok:a"])
        XCTAssertEqual(
            fold.summary, FoldSummary(title: "2 more", names: "· Copilot, Grok", providers: ["copilot", "grok"]))
        XCTAssertEqual(AccountFold.make(sections, state: state, strings: Build.russian.strings).summary?.title, "Ещё 2")
        let plain = try Build.state(accounts: [Build.accountJSON(id: "a")])
        let none = AccountFold.make(
            AccountSectionModel.sections(plain, formatter: Build.english), state: plain, strings: Build.english.strings)
        XCTAssertFalse(none.isFolding)
        XCTAssertNil(none.summary)
    }

    private func links() throws -> ProviderLinks {
        try Fixture.decode(
            ProviderLinks.self,
            json:
                #"{"status":"https://status.openai.com","dashboard":"https://chatgpt.com","usage":"https://chatgpt.com"}"#
        )
    }

    func testHeaderMenuListsActionsLinksAndSharing() throws {
        let state = try Build.full()
        let section = try XCTUnwrap(AccountSectionModel.sections(state, formatter: Build.english).first)
        var display = state.display
        display.starredAccounts = section.memberIDs
        let menu = HeaderMenuModel.make(
            section, providerName: "Codex", links: try links(), display: display, features: .current,
            strings: Build.english.strings)
        XCTAssertEqual(
            menu.groups.map { $0.map(\.title) },
            [
                ["Refresh Codex", "Hide from popup", "Always show"], ["Status page", "Open dashboard"],
                ["Share as image…", "Copy as text"],
            ])
        XCTAssertEqual(menu.groups[0][2].checked, true)
        XCTAssertEqual(menu.groups[1].map(\.detail), ["status.openai.com", "chatgpt.com"])
        XCTAssertEqual(menu.buttons.map(\.kind), [.status, .dashboard])
        XCTAssertEqual(menu.buttons.first?.tip, "Status page · status.openai.com")
        XCTAssertEqual(HeaderMenuModel.starredAfterToggle(section, display), [])
        XCTAssertEqual(HeaderMenuModel.starredAfterToggle(section, state.display), section.memberIDs)
        let legacy = HeaderMenuModel.make(
            section, providerName: "Codex", links: nil, display: state.display, features: .legacy,
            strings: Build.russian.strings)
        XCTAssertEqual(legacy.groups.first?.map(\.title), ["Обновить Codex", "Скрыть из окна"])
        XCTAssertEqual(legacy.buttons, [])
    }

    func testCompactLineDropsThePrefixAndMovesPaceToTheTip() throws {
        let window = try Build.window(
            id: "weekly", used: 68, tone: "warning", severity: "running_out", even: "50",
            runsOut: "\"2026-09-24T10:00:00Z\"", resetsAt: "\"2026-09-25T09:00:00Z\"")
        let display = try Build.display()
        let now = try Build.now()
        let row = QuotaRowModel.make(window, display: display, now: now, formatter: Build.english)
        let line = CompactLimitLine.make(
            row, resetsAt: window.resetsAt, display: display, now: now, formatter: Build.english)
        XCTAssertEqual(line.trailing, "1d 23h")
        XCTAssertTrue(line.flame)
        XCTAssertEqual(line.meterTip, "Resets in 1d 23h\nOver pace\nAt this pace: runs out in 1d 0h · resets in 1d 23h")
        let exact = try Build.display(resetFormat: "exact")
        let moment = CompactLimitLine.make(
            row, resetsAt: window.resetsAt, display: exact, now: now, formatter: Build.english)
        XCTAssertEqual(moment.trailing, "Fri 09:00")
        XCTAssertEqual(
            Build.english.compactMoment(try Fixture.timestamp("2026-09-23T14:30:00Z").date, now: now.date), "14:30")
        XCTAssertEqual(
            Build.english.compactMoment(try Fixture.timestamp("2026-10-05T14:30:00Z").date, now: now.date),
            "Oct 5 14:30")
    }

    func testCompactLayoutFollowsTheTokenTable() {
        XCTAssertEqual(PopupLayout.make(.normal), .normal)
        let compact = PopupLayout.make(.compact)
        XCTAssertTrue(compact.isCompact)
        XCTAssertEqual(compact.sectionGap, 6)
        XCTAssertEqual(compact.meterHeight, 4)
        XCTAssertEqual(compact.donutSize, 64)
        XCTAssertEqual(compact.size(13), 12)
        XCTAssertEqual(compact.trendScale, 14.0 / 18.0, accuracy: 1e-9)
        XCTAssertEqual(PopupLayout.normal.size(13), 13)
    }

    func testReadingTipsDescribeTheNextState() throws {
        let strings = Build.english.strings
        XCTAssertEqual(ReadingTips.value(try Build.display(), strings: strings), "Click to show used")
        XCTAssertEqual(
            ReadingTips.value(try Build.display(valueMode: "used"), strings: strings), "Click to show what's left")
        XCTAssertEqual(ReadingTips.reset(try Build.display(), strings: strings), "Click to show the reset time")
        XCTAssertEqual(
            ReadingTips.reset(try Build.display(resetFormat: "exact"), strings: strings), "Click to show the countdown")
    }

    func testEveryNewStringIsTranslated() {
        for key in PopupExtraText.allCases {
            XCTAssertFalse(key.translations.english.isEmpty, "\(key)")
            XCTAssertFalse(key.translations.russian.isEmpty, "\(key)")
        }
        for key in StatusText.allCases {
            XCTAssertFalse(key.translations.english.isEmpty || key.translations.russian.isEmpty, "\(key)")
        }
        XCTAssertEqual(Build.russian.strings.fill(.models, count: 3), "3 модели")
        XCTAssertEqual(Build.english.strings.fill(.otherProjects, count: 1), "· 1 project")
    }

    func testToasts() {
        let strings = Build.english.strings
        XCTAssertEqual(
            PopupToast.imageShared(id: 1, saved: true, strings: strings).detail, "saved to Pictures/Headroom")
        XCTAssertNil(PopupToast.imageShared(id: 2, saved: false, strings: strings).detail)
        XCTAssertEqual(PopupToast.shareFailed(id: 3, strings: strings).kind, .failure)
        XCTAssertEqual(PopupToast.textCopied(id: 4, strings: Build.russian.strings).title, "Скопировано текстом")
    }

    func testLinksComeFromTheLoadedProviderList() throws {
        let payload = try Fixture.decode(ProvidersPayload.self, "providers")
        let list = ProviderList.loaded(payload.providers)
        XCTAssertEqual(list.links(for: "codex"), payload.providers.first { $0.id == "codex" }?.links)
        XCTAssertNil(list.links(for: "nope"))
        XCTAssertNil(ProviderList.loading.links(for: "codex"))
    }
}
