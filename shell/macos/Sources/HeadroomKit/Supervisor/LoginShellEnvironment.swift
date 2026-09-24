import Foundation

public enum LoginShellEnvironment {
    public static let beginMarker = "__HEADROOM_ENV_BEGIN__"
    public static let endMarker = "__HEADROOM_ENV_END__"
    public static let timeout: Duration = .seconds(5)

    static let exactKeys: Set<String> = [
        "PATH", "CODEX_HOME", "CLAUDE_CONFIG_DIR", "GROK_HOME", "CLINE_DIR", "GH_CONFIG_DIR", "LANG",
    ]
    static let keyPrefixes = ["XDG_", "LC_"]

    static let script = "printf '\\n%s\\n' \(beginMarker); /usr/bin/env -0; printf '%s' \(endMarker)"

    public static func capture(
        shell: String, runner: any CommandRunning = CommandRunner(), timeout: Duration = timeout
    ) async -> [String: String] {
        let output = try? await runner.output(
            of: URL(fileURLWithPath: shell), arguments: ["-i", "-l", "-c", script], until: endMarker,
            timeout: timeout)
        guard let output else { return [:] }
        return relevant(parse(output))
    }

    public static func parse(_ output: String) -> [String: String] {
        guard let block = markedBlock(output) else { return [:] }
        var environment: [String: String] = [:]
        for entry in block.split(separator: "\0") {
            guard let (key, value) = assignment(entry) else { continue }
            environment[key] = value
        }
        return environment
    }

    private static func markedBlock(_ output: String) -> Substring? {
        guard let end = output.range(of: endMarker, options: .backwards),
            let begin = output.range(
                of: "\n\(beginMarker)\n", options: .backwards, range: output.startIndex..<end.lowerBound)
        else { return nil }
        return output[begin.upperBound..<end.lowerBound]
    }

    public static func relevant(_ environment: [String: String]) -> [String: String] {
        environment.filter { key, _ in isRelevant(key) }
    }

    static func isRelevant(_ key: String) -> Bool {
        exactKeys.contains(key) || keyPrefixes.contains { key.hasPrefix($0) }
    }

    private static func assignment(_ entry: Substring) -> (String, String)? {
        guard let equals = entry.firstIndex(of: "="), equals > entry.startIndex else { return nil }
        let key = String(entry[..<equals])
        guard key.allSatisfy({ $0 == "_" || ($0.isASCII && ($0.isLetter || $0.isNumber)) }) else {
            return nil
        }
        return (key, String(entry[entry.index(after: equals)...]))
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
