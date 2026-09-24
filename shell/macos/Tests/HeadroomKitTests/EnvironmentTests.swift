import Foundation
import XCTest

@testable import HeadroomKit

final class EnvironmentTests: XCTestCase {
    func testParsesOnlyBetweenMarkers() {
        let output =
            "Welcome to zsh!\nPATH=/bogus\n\n__HEADROOM_ENV_BEGIN__\n"
            + "PATH=/opt/homebrew/bin:/usr/bin\0CODEX_HOME=/Users/ada/.codex\0MULTI=line one\ncontinued line\0"
            + "EMPTY=\0=broken\0__HEADROOM_ENV_END__background noise\nPATH=/late\0"
        XCTAssertEqual(
            LoginShellEnvironment.parse(output),
            [
                "PATH": "/opt/homebrew/bin:/usr/bin", "CODEX_HOME": "/Users/ada/.codex",
                "MULTI": "line one\ncontinued line", "EMPTY": "",
            ])
        XCTAssertEqual(LoginShellEnvironment.parse("PATH=/x"), [:])
        XCTAssertEqual(LoginShellEnvironment.parse("\n__HEADROOM_ENV_BEGIN__\nPATH=/x\0"), [:])
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
        let runner = ScriptedRunner(.success("\n__HEADROOM_ENV_BEGIN__\nPATH=/p\0X=1\0__HEADROOM_ENV_END__"))
        let environment = await LoginShellEnvironment.capture(shell: "/bin/zsh", runner: runner)
        XCTAssertEqual(environment, ["PATH": "/p"])
        XCTAssertEqual(runner.calls.first?.arguments.prefix(3), ["-i", "-l", "-c"])
    }

    func testCaptureFailureYieldsEmptyEnvironment() async {
        let timedOut = ScriptedRunner(.failure(.timedOut))
        let nothing = await LoginShellEnvironment.capture(shell: "/bin/zsh", runner: timedOut)
        XCTAssertEqual(nothing, [:])
        let unterminated = ScriptedRunner(.success("\n__HEADROOM_ENV_BEGIN__\nPATH=/p\0"))
        let partial = await LoginShellEnvironment.capture(shell: "/bin/zsh", runner: unterminated)
        XCTAssertEqual(partial, [:])
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

    func testOutputStopsAtTheTerminatorWhileABackgroundChildKeepsStdoutOpen() async throws {
        let started = ContinuousClock.now
        let output = try await CommandRunner().output(
            of: URL(fileURLWithPath: "/bin/sh"), arguments: ["-c", "sleep 6 & printf 'done END'; wait"],
            until: "END", timeout: .seconds(10))
        XCTAssertEqual(output, "done END")
        XCTAssertLessThan(ContinuousClock.now - started, .seconds(5))
    }

    func testCaptureSurvivesAShellThatLeavesABackgroundJobOnStdout() async throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let pidFile = directory.appendingPathComponent("sleeper.pid")
        let shell = try FakeLoginShell.write(in: directory, pidFile: pidFile)
        defer { FakeLoginShell.killSleeper(pidFile: pidFile) }
        let started = ContinuousClock.now
        let environment = await LoginShellEnvironment.capture(shell: shell.path, timeout: .seconds(10))
        XCTAssertLessThan(ContinuousClock.now - started, .seconds(5))
        XCTAssertEqual(environment["PATH"], "/fake/login/bin")
        XCTAssertEqual(environment["CODEX_HOME"], "/fake/codex\nhome")
        XCTAssertNil(environment["NOISE"])
    }
}

private enum FakeLoginShell {
    static func write(in directory: URL, pidFile: URL) throws -> URL {
        let shell = directory.appendingPathComponent("fake-zsh")
        let script = """
            #!/bin/sh
            echo "rc noise"
            sleep 30 &
            echo $! > '\(pidFile.path)'
            export PATH=/fake/login/bin
            export CODEX_HOME='/fake/codex
            home'
            export NOISE=1
            shift 3
            exec /bin/sh -c "$1"
            """
        try Data(script.utf8).write(to: shell)
        try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: shell.path)
        return shell
    }

    static func killSleeper(pidFile: URL) {
        guard let text = try? String(contentsOf: pidFile, encoding: .utf8),
            let pid = Int32(text.trimmingCharacters(in: .whitespacesAndNewlines))
        else { return }
        _ = kill(pid, SIGKILL)
    }
}

private final class ScriptedRunner: CommandRunning, @unchecked Sendable {
    struct Call {
        let executable: URL
        let arguments: [String]
    }

    private let result: Result<String, CommandError>
    private let lock = NSLock()
    private var recorded: [Call] = []

    init(_ result: Result<String, CommandError>) {
        self.result = result
    }

    var calls: [Call] { lock.withLock { recorded } }

    func run(
        _ executable: URL, arguments: [String], environment: [String: String]?, timeout: Duration
    ) async throws(CommandError) -> CommandOutput {
        CommandOutput(status: 0, stdout: try output(of: executable, arguments: arguments))
    }

    func output(
        of executable: URL, arguments: [String], until terminator: String, timeout: Duration
    ) async throws(CommandError) -> String {
        try output(of: executable, arguments: arguments)
    }

    private func output(of executable: URL, arguments: [String]) throws(CommandError) -> String {
        lock.withLock { recorded.append(Call(executable: executable, arguments: arguments)) }
        return try result.get()
    }
}
