import Foundation
import XCTest

@testable import HeadroomKit

@MainActor
final class AddAccountSessionTests: XCTestCase {
    private let provider = ProviderInfo(
        id: "codex", displayName: "Codex", addAccount: [.cliLogin(program: "codex")], multiAccount: true,
        localUsage: true, links: nil)

    func testSignInCollectsUrlCodeAndLog() {
        let launcher = FakeHelperLauncher()
        let session = AddAccountSession(provider: provider, launcher: launcher)
        session.start(label: "Work")
        XCTAssertEqual(launcher.commands, [.add(provider: "codex", label: "Work", apiKeyOnStdin: false)])
        session.receive(.event(.started))
        session.receive(.event(.output("Open this page")))
        session.receive(.event(.url("https://auth.example/a")))
        session.receive(.event(.url("https://auth.example/b")))
        session.receive(.event(.output("Enter the code WXYZ-1234")))
        var expected = SignInProgress()
        expected.url = "https://auth.example/a"
        expected.deviceCode = "WXYZ-1234"
        expected.log = [
            "Open this page", "https://auth.example/a", "https://auth.example/b", "Enter the code WXYZ-1234",
        ]
        XCTAssertEqual(session.phase, .running(expected))
        session.sendCode("  abc \n")
        session.sendCode("   ")
        XCTAssertEqual(launcher.handle.lines, ["abc"])
        session.receive(.event(.done(accountID: "codex:a")))
        session.receive(.exited(0))
        XCTAssertEqual(session.phase, .done(accountID: "codex:a"))
    }

    func testApiKeyGoesToStdinOnly() {
        let launcher = FakeHelperLauncher()
        let session = AddAccountSession(provider: provider, launcher: launcher)
        session.start(label: "", apiKey: "sk-secret")
        XCTAssertEqual(launcher.commands, [.add(provider: "codex", label: "", apiKeyOnStdin: true)])
        XCTAssertEqual(launcher.handle.lines, ["sk-secret"])
        XCTAssertTrue(launcher.handle.inputClosed)
        XCTAssertFalse(launcher.commands.flatMap(\.arguments).contains("sk-secret"))
    }

    func testErrorEventFailsAndStopsTheProcess() {
        let launcher = FakeHelperLauncher()
        let session = AddAccountSession(provider: provider, launcher: launcher)
        session.start(label: "")
        session.receive(.event(.error("key rejected")))
        session.receive(.exited(1))
        XCTAssertEqual(session.phase, .failed(.message("key rejected")))
        XCTAssertTrue(launcher.handle.wasCancelled)
    }

    func testExitWithoutEventsDecidesByStatus() {
        let session = AddAccountSession(provider: provider, launcher: FakeHelperLauncher())
        session.start(label: "")
        session.receive(.exited(3))
        XCTAssertEqual(session.phase, .failed(.exitStatus(3)))
        session.cancel()
        XCTAssertEqual(session.phase, .form)
        session.start(label: "")
        session.receive(.exited(0))
        XCTAssertEqual(session.phase, .done(accountID: nil))
    }

    func testLaunchFailureAndCancel() {
        let failing = AddAccountSession(provider: provider, launcher: FakeHelperLauncher(failure: .missingHelper))
        failing.start(label: "")
        XCTAssertEqual(failing.phase, .failed(.missingHelper))
        let launcher = FakeHelperLauncher()
        let session = AddAccountSession(provider: provider, launcher: launcher)
        session.start(label: "")
        session.cancel()
        XCTAssertEqual(session.phase, .form)
        XCTAssertTrue(launcher.handle.wasCancelled)
        session.receive(.event(.done(accountID: "codex:late")))
        XCTAssertEqual(session.phase, .form)
    }

    func testEventsArriveThroughTheProcessStream() async {
        let launcher = FakeHelperLauncher()
        let session = AddAccountSession(provider: provider, launcher: launcher)
        session.start(label: "")
        launcher.handle.emit(.event(.output("hello")), .event(.done(accountID: "codex:b")), .exited(0))
        launcher.handle.end()
        for _ in 0..<100 where session.isRunning { await Task.yield() }
        XCTAssertEqual(session.phase, .done(accountID: "codex:b"))
    }

    func testLogIsBounded() {
        var progress = SignInProgress()
        for index in 0..<(SignInProgress.logLimit + 5) { progress.record("line \(index)") }
        XCTAssertEqual(progress.log.count, SignInProgress.logLimit)
        XCTAssertEqual(progress.log.first, "line 5")
    }

    func testRemovalReportsTheCliError() async {
        let launcher = FakeHelperLauncher()
        launcher.handle.emit(.event(.error("no daemon")), .exited(1))
        launcher.handle.end()
        let outcome = await AccountRemoval.remove(accountID: "codex:a", launcher: launcher)
        XCTAssertEqual(launcher.commands, [.remove(accountID: "codex:a")])
        guard case .failure(let failure) = outcome else { return XCTFail("expected failure") }
        XCTAssertEqual(failure, .message("no daemon"))
    }

    func testRemovalSucceedsOnDone() async {
        let launcher = FakeHelperLauncher()
        launcher.handle.emit(.event(.done(accountID: "codex:a")), .exited(0))
        launcher.handle.end()
        let outcome = await AccountRemoval.remove(accountID: "codex:a", launcher: launcher)
        XCTAssertNoThrow(try outcome.get())
    }
}
