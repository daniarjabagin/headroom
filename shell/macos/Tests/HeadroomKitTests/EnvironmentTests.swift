import Foundation
import XCTest

@testable import HeadroomKit

final class EnvironmentTests: XCTestCase {
    func testParsesOnlyAfterMarker() {
        let output = """
            Welcome to zsh!
            PATH=/bogus
            __HEADROOM_ENVIRONMENT__
            PATH=/opt/homebrew/bin:/usr/bin
            CODEX_HOME=/Users/ada/.codex
            MULTI=line one
            continued line
            EMPTY=
            =broken
            """
        XCTAssertEqual(
            LoginShellEnvironment.parse(output),
            ["PATH": "/opt/homebrew/bin:/usr/bin", "CODEX_HOME": "/Users/ada/.codex", "MULTI": "line one", "EMPTY": ""])
        XCTAssertEqual(LoginShellEnvironment.parse("PATH=/x"), [:])
    }

    func testKeepsOnlyRelevantVariables() {
        let captured = [
            "PATH": "/p", "CODEX_HOME": "/c", "CLAUDE_CONFIG_DIR": "/cl", "GROK_HOME": "/g", "CLINE_DIR": "/cn",
            "GH_CONFIG_DIR": "/gh", "XDG_CONFIG_HOME": "/x", "LANG": "ru_RU.UTF-8", "LC_ALL": "C",
            "OPENAI_API_KEY": "secret", "HOME": "/h", "SSH_AUTH_SOCK": "/s",
        ]
        XCTAssertEqual(
            Set(LoginShellEnvironment.relevant(captured).keys),
            [
                "PATH", "CODEX_HOME", "CLAUDE_CONFIG_DIR", "GROK_HOME", "CLINE_DIR", "GH_CONFIG_DIR", "XDG_CONFIG_HOME",
                "LANG", "LC_ALL",
            ])
    }

    func testDaemonEnvironmentOverlaysLoginShellAndDerivesLang() {
        let built = DaemonEnvironment.build(
            base: ["PATH": "/usr/bin", "HOME": "/Users/ada"], loginShell: ["PATH": "/opt/homebrew/bin"],
            preferredLanguage: "ru-RU")
        XCTAssertEqual(built, ["PATH": "/opt/homebrew/bin", "HOME": "/Users/ada", "LANG": "ru_RU.UTF-8"])
        let kept = DaemonEnvironment.build(base: ["LANG": "en_GB.UTF-8"], loginShell: [:], preferredLanguage: "ru")
        XCTAssertEqual(kept["LANG"], "en_GB.UTF-8")
    }

    func testHelperEnvironmentAddsTheSocket() {
        let helper = DaemonEnvironment.helper(
            daemon: ["PATH": "/opt/homebrew/bin", "HEADROOM_SOCKET": "/old"], socketPath: "/tmp/h/daemon.sock")
        XCTAssertEqual(helper, ["PATH": "/opt/homebrew/bin", "HEADROOM_SOCKET": "/tmp/h/daemon.sock"])
    }

    func testPosixLocaleFromLanguageTags() {
        XCTAssertEqual(DaemonEnvironment.posixLocale(fromLanguageTag: "ru-RU"), "ru_RU.UTF-8")
        XCTAssertEqual(DaemonEnvironment.posixLocale(fromLanguageTag: "en"), "en.UTF-8")
        XCTAssertEqual(DaemonEnvironment.posixLocale(fromLanguageTag: "zh-Hans-CN"), "zh_CN.UTF-8")
    }

    func testHelperVersionParsing() {
        XCTAssertEqual(HelperVersion.parse("headroom 0.3.0\n"), "0.3.0")
        XCTAssertNil(HelperVersion.parse("something else"))
        XCTAssertNil(HelperVersion.parse(""))
    }

    func testCaptureRunsShellAndFilters() async {
        let runner = ScriptedRunner(
            output: CommandOutput(status: 0, stdout: "__HEADROOM_ENVIRONMENT__\nPATH=/p\nX=1\n"))
        let environment = await LoginShellEnvironment.capture(shell: "/bin/zsh", runner: runner)
        XCTAssertEqual(environment, ["PATH": "/p"])
        XCTAssertEqual(runner.calls.first?.arguments.prefix(3), ["-i", "-l", "-c"])
    }

    func testCaptureFailureYieldsEmptyEnvironment() async {
        let runner = ScriptedRunner(output: CommandOutput(status: 1, stdout: "__HEADROOM_ENVIRONMENT__\nPATH=/p\n"))
        let environment = await LoginShellEnvironment.capture(shell: "/bin/zsh", runner: runner)
        XCTAssertEqual(environment, [:])
    }

    func testCommandRunnerCapturesOutputAndTimesOut() async throws {
        let runner = CommandRunner()
        let shell = URL(fileURLWithPath: "/bin/sh")
        let output = try await runner.run(shell, arguments: ["-c", "echo hi"], environment: nil, timeout: .seconds(5))
        XCTAssertEqual(output, CommandOutput(status: 0, stdout: "hi\n"))
        do {
            _ = try await runner.run(
                shell, arguments: ["-c", "sleep 5"], environment: nil, timeout: .milliseconds(100))
            XCTFail("expected timeout")
        } catch {
            XCTAssertEqual(error, .timedOut)
        }
    }
}

private final class ScriptedRunner: CommandRunning, @unchecked Sendable {
    struct Call {
        let executable: URL
        let arguments: [String]
    }

    private let output: CommandOutput
    private let lock = NSLock()
    private var recorded: [Call] = []

    init(output: CommandOutput) {
        self.output = output
    }

    var calls: [Call] { lock.withLock { recorded } }

    func run(
        _ executable: URL, arguments: [String], environment: [String: String]?, timeout: Duration
    ) async throws(CommandError) -> CommandOutput {
        lock.withLock { recorded.append(Call(executable: executable, arguments: arguments)) }
        return output
    }
}
