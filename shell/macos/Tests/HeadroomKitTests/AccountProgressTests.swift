import Foundation
import XCTest

@testable import HeadroomKit

final class AccountProgressTests: XCTestCase {
    func testAddArgumentsKeepTheKeyOffTheCommandLine() {
        XCTAssertEqual(
            AccountCommand.add(provider: "codex", label: "  Work ", apiKeyOnStdin: false).arguments,
            ["accounts", "add", "codex", "--label=Work", "--progress", "json"])
        XCTAssertEqual(
            AccountCommand.add(provider: "openrouter", label: "", apiKeyOnStdin: true).arguments,
            ["accounts", "add", "openrouter", "--api-key-stdin", "--progress", "json"])
        XCTAssertEqual(
            AccountCommand.remove(accountID: "codex:1a").arguments,
            ["accounts", "remove", "codex:1a", "--yes", "--progress", "json"])
    }

    func testParsesDocumentedEvents() {
        XCTAssertEqual(AccountProgressEvent.parse(#"{"event":"started","provider":"codex","home":"/h"}"#), .started)
        XCTAssertEqual(
            AccountProgressEvent.parse(#"{"event":"url","url":"https://auth.example/"}"#), .url("https://auth.example/")
        )
        XCTAssertEqual(
            AccountProgressEvent.parse(#"{"event":"output","line":"Paste code: "}"#), .output("Paste code: "))
        XCTAssertEqual(
            AccountProgressEvent.parse(#"{"event":"done","account_id":"codex:a","label":null}"#),
            .done(accountID: "codex:a"))
        XCTAssertEqual(AccountProgressEvent.parse(#"{"event":"error","message":"cancelled"}"#), .error("cancelled"))
        XCTAssertEqual(AccountProgressEvent.parse(#"{"event":"error"}"#), .error("unknown error"))
    }

    func testOtherLinesBecomeOutput() {
        XCTAssertNil(AccountProgressEvent.parse("   "))
        XCTAssertEqual(AccountProgressEvent.parse("warning: plain text"), .output("warning: plain text"))
        XCTAssertEqual(AccountProgressEvent.parse(#"{"event":"future"}"#), .output(#"{"event":"future"}"#))
        XCTAssertEqual(AccountProgressEvent.parse(#"{"event":"url"}"#), .output(#"{"event":"url"}"#))
        XCTAssertEqual(AccountProgressEvent.parse("[1,2]"), .output("[1,2]"))
    }

    func testDeviceCodeDetection() {
        XCTAssertEqual(DeviceCode.find(in: "! First copy your one-time code: 3F2A-9BC1"), "3F2A-9BC1")
        XCTAssertEqual(DeviceCode.find(in: "Enter code ABCD-EFGH at https://example.com/device"), "ABCD-EFGH")
        XCTAssertNil(DeviceCode.find(in: "Open https://example.com/a-b and sign in"))
        XCTAssertNil(DeviceCode.find(in: "code: abcd-efgh"))
        XCTAssertNil(DeviceCode.find(in: "3F2A-9BC1 without the keyword"))
    }

    func testFailureTexts() {
        let strings = UIStrings(language: .en)
        XCTAssertEqual(HelperFailure.exitStatus(2).text(strings), "headroom exited with status 2")
        XCTAssertEqual(HelperFailure.message("no luck").text(strings), "no luck")
        XCTAssertEqual(
            HelperFailure.missingHelper.text(UIStrings(language: .ru)),
            "В приложении нет помощника headroom. Переустановите Headroom.")
    }
}
