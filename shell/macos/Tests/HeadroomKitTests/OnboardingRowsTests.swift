import XCTest

@testable import HeadroomKit

final class OnboardingRowsTests: XCTestCase {
    private func provider(_ id: String, _ methods: [AddAccountMethod]) -> ProviderInfo {
        ProviderInfo(
            id: id, displayName: id.capitalized, addAccount: methods, multiAccount: true, localUsage: true, links: nil)
    }

    private func accounts() throws -> [Account] {
        try Build.state(accounts: [
            Build.accountJSON(id: "cursor:a", provider: "cursor", status: "signed_out"),
            Build.accountJSON(id: "claude:a", provider: "claude", email: "\"me@example.org\"", plan: "\"Max 5x\""),
            Build.accountJSON(id: "codex:a", provider: "codex", plan: "null", hidden: true),
        ]).accounts
    }

    func testFoundAccountsComeFirstThenSignInThenMissingTools() throws {
        let providers = [
            provider("claude", [.cliLogin(program: "claude")]), provider("copilot", [.cliLogin(program: "gh")]),
            provider("gemini", [.autoDetect(reason: "")]),
            provider("openrouter", [.apiKey(label: "Key", consoleURL: "", hint: "")]),
        ]
        let rows = OnboardingRows.rows(accounts: try accounts(), providers: providers, strings: Build.english.strings)
        XCTAssertEqual(rows.map(\.id), ["claude:a", "codex:a", "cursor:a", "provider:copilot", "provider:gemini"])
        XCTAssertEqual(
            rows.map(\.kind),
            [
                .tracked(accountID: "claude:a", shown: true), .tracked(accountID: "codex:a", shown: false),
                .signIn(provider: "cursor"), .notInstalled, .notInstalled,
            ])
        XCTAssertEqual(
            rows.map(\.subtitle),
            [
                "Signed in · Max 5x · me@example.org", "Signed in", "Found, not signed in", "Not installed",
                "Not installed",
            ])
        XCTAssertEqual(rows[3].title, "Copilot")
    }

    func testRussianSubtitles() throws {
        let rows = OnboardingRows.rows(accounts: try accounts(), providers: [], strings: Build.russian.strings)
        XCTAssertEqual(
            rows.map(\.subtitle),
            ["Вход выполнен · Max 5x · me@example.org", "Вход выполнен", "Найден, вход не выполнен"])
    }

    func testNothingFoundGivesNoRows() {
        XCTAssertEqual(OnboardingRows.rows(accounts: [], providers: [], strings: Build.english.strings), [])
    }
}
