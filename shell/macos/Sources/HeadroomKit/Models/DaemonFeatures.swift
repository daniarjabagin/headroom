public struct DaemonFeatures: Sendable, Hashable {
    public static let legacy = DaemonFeatures(release06: false)
    public static let current = DaemonFeatures(release06: true)
    public static let release06Version = [0, 6, 0]

    public let release06: Bool

    public init(release06: Bool) {
        self.release06 = release06
    }

    public init(state: DaemonState) {
        release06 = state.reportsPanelItems || Self.isRelease06(appVersion: state.appVersion)
    }

    static func isRelease06(appVersion: String?) -> Bool {
        guard let appVersion, let parts = numericParts(appVersion) else { return false }
        return !parts.lexicographicallyPrecedes(release06Version)
    }

    private static func numericParts(_ version: String) -> [Int]? {
        let core = version.split(separator: "-", maxSplits: 1).first.map(String.init) ?? version
        let parts = core.split(separator: ".", omittingEmptySubsequences: false).map { Int($0) }
        guard parts.count == 3 else { return nil }
        let numbers = parts.compactMap { $0 }
        return numbers.count == 3 ? numbers : nil
    }
}
