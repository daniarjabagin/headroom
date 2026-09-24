public enum SocketPath {
    public static let overrideVariable = "HEADROOM_SOCKET"
    public static let maximumLength = 103
    public static let socketName = "daemon.sock"

    public static func resolve(environment: [String: String], home: String, uid: UInt32) -> String {
        if let override = environment[overrideVariable], !override.isEmpty {
            return override
        }
        let preferred = defaultPath(home: home)
        guard preferred.utf8.count > maximumLength else { return preferred }
        return fallbackPath(temporaryDirectory: environment["TMPDIR"], uid: uid)
    }

    public static func defaultPath(home: String) -> String {
        joined(joined(home, "Library/Application Support/Headroom"), socketName)
    }

    static func fallbackPath(temporaryDirectory: String?, uid: UInt32) -> String {
        let directory = temporaryDirectory.flatMap { $0.isEmpty ? nil : $0 } ?? "/tmp"
        return joined(joined(directory, "headroom-\(uid)"), socketName)
    }

    private static func joined(_ directory: String, _ name: String) -> String {
        directory.hasSuffix("/") ? directory + name : directory + "/" + name
    }
}
