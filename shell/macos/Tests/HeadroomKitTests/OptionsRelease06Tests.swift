import Foundation
import XCTest

@testable import HeadroomKit

final class OptionsRelease06Tests: XCTestCase {
    private let english = UIStrings(language: .en)
    private let russian = UIStrings(language: .ru)

    func testEveryOptionHasATitleInBothLanguages() {
        assertTitles(Density.allCases.map { ($0.title(english), $0.title(russian)) })
        assertTitles(TimeFormat.allCases.map { ($0.title(english), $0.title(russian)) })
        assertTitles(PanelMode.allCases.map { ($0.title(english), $0.title(russian)) })
        assertTitles(PanelIndicator.allCases.map { ($0.title(english), $0.title(russian)) })
        assertTitles(PanelLabel.allCases.map { ($0.title(english), $0.title(russian)) })
        assertTitles(SpendPeriodPreference.allCases.map { ($0.title(english), $0.title(russian)) })
        assertTitles(SpendUnit.allCases.map { ($0.title(english), $0.title(russian)) })
        assertTitles(SpendUnit.allCases.map { ($0.detail(english), $0.detail(russian)) })
        assertTitles(SpendBreakdown.allCases.map { ($0.title(english), $0.title(russian)) })
        assertTitles(LogLevel.allCases.map { ($0.title(english), $0.title(russian)) })
    }

    func testSampleTitles() {
        XCTAssertEqual(TimeFormat.twelveHour.title(english), "12-hour")
        XCTAssertEqual(TimeFormat.twentyFourHour.title(russian), "24-часовой")
        XCTAssertEqual(
            SpendPeriodPreference.allCases.map { $0.title(russian) }, ["Сегодня", "Вчера", "7 дней", "30 дней"])
        XCTAssertEqual(SpendUnit.costPerMTok.title(english), "Cost per MTok")
        XCTAssertEqual(PanelLabel.none.title(english), "No label")
        XCTAssertEqual(PanelLabel.window.title(english), "Provider + limit")
        XCTAssertEqual(LogLevel.debug.title(russian), "Отладка")
    }

    func testThresholdChoicesKeepACustomValue() {
        XCTAssertEqual(SettingsOptions.thresholds(current: 10), [5, 10, 20, 30])
        XCTAssertEqual(SettingsOptions.thresholds(current: 15), [5, 10, 15, 20, 30])
        XCTAssertEqual(SettingsOptions.providerThresholds(current: nil), [nil, 0, 5, 10, 20, 30])
        XCTAssertEqual(SettingsOptions.providerThresholds(current: 0), [nil, 0, 5, 10, 20, 30])
        XCTAssertEqual(SettingsOptions.providerThresholds(current: 45), [nil, 0, 5, 10, 20, 30, 45])
    }

    func testThresholdLabels() {
        XCTAssertEqual(SettingsOptions.thresholdLabel(nil, general: 10, strings: english), "Default (10%)")
        XCTAssertEqual(SettingsOptions.thresholdLabel(0, general: 10, strings: english), "Off")
        XCTAssertEqual(SettingsOptions.thresholdLabel(20, general: 10, strings: english), "Under 20% left")
        XCTAssertEqual(SettingsOptions.thresholdLabel(20, general: 10, strings: russian), "Осталось меньше 20%")
        XCTAssertEqual(SettingsOptions.thresholdLabel(nil, general: 5, strings: russian), "По умолчанию (5%)")
    }

    func testServiceLogPathAndDisplayForm() {
        let home = URL(fileURLWithPath: "/Users/ada")
        let log = LogFile.serviceLog(home: home)
        XCTAssertEqual(log.path, "/Users/ada/Library/Logs/Headroom/headroom.log")
        XCTAssertEqual(LogFile.displayPath(log, home: home), "~/Library/Logs/Headroom/headroom.log")
        XCTAssertEqual(LogFile.displayPath(log, home: URL(fileURLWithPath: "/Users/ad")), log.path)
        XCTAssertEqual(LogFile.displayPath(URL(fileURLWithPath: "/tmp/x.log"), home: home), "/tmp/x.log")
        XCTAssertNotEqual(LogFile.serviceLog(home: home), LogFile.daemonLog(home: home))
    }

    private func assertTitles(_ titles: [(String, String)], file: StaticString = #filePath, line: UInt = #line) {
        for (englishTitle, russianTitle) in titles {
            XCTAssertFalse(englishTitle.isEmpty, file: file, line: line)
            XCTAssertNotEqual(englishTitle, russianTitle, file: file, line: line)
        }
        XCTAssertEqual(Set(titles.map(\.0)).count, titles.count, "duplicate English titles", file: file, line: line)
        XCTAssertEqual(Set(titles.map(\.1)).count, titles.count, "duplicate Russian titles", file: file, line: line)
    }
}
