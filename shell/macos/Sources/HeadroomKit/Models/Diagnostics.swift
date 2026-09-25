public struct Diagnostics: Decodable, Sendable, Hashable {
    public let appVersion: String
    public let os: String?
    public let desktop: String?
    public let uptimeSecs: Double
    public let transports: [String]
    public let logLevel: String
    public let logLevelSource: String
    public let logFile: String?
    public let text: String

    enum CodingKeys: String, CodingKey {
        case os, desktop, transports, text
        case appVersion = "app_version"
        case uptimeSecs = "uptime_secs"
        case logLevel = "log_level"
        case logLevelSource = "log_level_source"
        case logFile = "log_file"
    }
}
