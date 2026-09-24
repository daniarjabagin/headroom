import Foundation

public enum LoginShellEnvironment {
    public static let marker = "__HEADROOM_ENVIRONMENT__"
    public static let timeout: Duration = .seconds(5)

    static let exactKeys: Set<String> = [
        "PATH", "CODEX_HOME", "CLAUDE_CONFIG_DIR", "GROK_HOME", "CLINE_DIR", "GH_CONFIG_DIR", "LANG",
    ]
    static let keyPrefixes = ["XDG_", "LC_"]

    public static func capture(
        shell: String, runner: any CommandRunning = CommandRunner(), timeout: Duration = timeout
    ) async -> [String: String] {
        let script = "printf '%s\\n' \(marker); /usr/bin/env"
        let output = try? await runner.run(
            URL(fileURLWithPath: shell), arguments: ["-i", "-l", "-c", script], environment: nil,
            timeout: timeout)
        guard let output, output.status == 0 else { return [:] }
        return relevant(parse(output.stdout))
    }

    public static func parse(_ output: String) -> [String: String] {
        let lines = output.split(separator: "\n", omittingEmptySubsequences: false)
        guard let start = lines.lastIndex(where: { $0 == marker }) else { return [:] }
        var environment: [String: String] = [:]
        for line in lines[lines.index(after: start)...] {
            guard let (key, value) = assignment(line) else { continue }
            environment[key] = value
        }
        return environment
    }

    public static func relevant(_ environment: [String: String]) -> [String: String] {
        environment.filter { key, _ in isRelevant(key) }
    }

    static func isRelevant(_ key: String) -> Bool {
        exactKeys.contains(key) || keyPrefixes.contains { key.hasPrefix($0) }
    }

    private static func assignment(_ line: Substring) -> (String, String)? {
        guard let equals = line.firstIndex(of: "="), equals > line.startIndex else { return nil }
        let key = String(line[..<equals])
        guard key.allSatisfy({ $0 == "_" || ($0.isASCII && ($0.isLetter || $0.isNumber)) }) else {
            return nil
        }
        return (key, String(line[line.index(after: equals)...]))
    }
}

public enum DaemonEnvironment {
    public static func build(
        base: [String: String], loginShell: [String: String], preferredLanguage: String?
    ) -> [String: String] {
        var environment = base.merging(loginShell) { _, login in login }
        if environment["LANG", default: ""].isEmpty, let preferredLanguage {
            environment["LANG"] = posixLocale(fromLanguageTag: preferredLanguage)
        }
        return environment
    }

    public static func helper(daemon environment: [String: String], socketPath: String) -> [String: String] {
        environment.merging([SocketPath.overrideVariable: socketPath]) { _, socket in socket }
    }

    public static func posixLocale(fromLanguageTag tag: String) -> String {
        let parts = tag.split(whereSeparator: { $0 == "-" || $0 == "_" }).map(String.init)
        let language = parts.first?.lowercased() ?? "en"
        let region = parts.dropFirst().first { $0.count == 2 && $0.allSatisfy(\.isLetter) }
        guard let region else { return "\(language).UTF-8" }
        return "\(language)_\(region.uppercased()).UTF-8"
    }
}

public enum HelperVersion {
    public static func parse(_ output: String) -> String? {
        let words = output.split(whereSeparator: \.isWhitespace)
        guard words.count >= 2, words[0] == "headroom" else { return nil }
        return String(words[1])
    }

    public static func query(
        helper: URL, runner: any CommandRunning = CommandRunner()
    ) async throws(CommandError) -> String? {
        let output = try await runner.run(helper, arguments: ["--version"], environment: nil, timeout: .seconds(5))
        guard output.status == 0 else { return nil }
        return parse(output.stdout)
    }
}
