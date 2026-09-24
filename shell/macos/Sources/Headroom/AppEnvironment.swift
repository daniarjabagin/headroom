#if canImport(AppKit)
    import Foundation
    import HeadroomKit

    struct AppEnvironment {
        let helper: URL?
        let socketPath: String
        let logFile: URL
        let bundledVersion: String?
        let processEnvironment: [String: String]

        static func current() -> AppEnvironment {
            let bundle = Bundle.main
            let processEnvironment = ProcessInfo.processInfo.environment
            let home = URL(fileURLWithPath: NSHomeDirectory())
            return AppEnvironment(
                helper: bundledHelper(in: bundle),
                socketPath: SocketPath.resolve(environment: processEnvironment, home: home.path, uid: getuid()),
                logFile: LogFile.daemonLog(home: home),
                bundledVersion: bundle.infoDictionary?["CFBundleShortVersionString"] as? String,
                processEnvironment: processEnvironment)
        }

        var loginShell: String {
            processEnvironment["SHELL"].flatMap { $0.isEmpty ? nil : $0 } ?? "/bin/zsh"
        }

        private static func bundledHelper(in bundle: Bundle) -> URL? {
            let helper = bundle.bundleURL.appendingPathComponent("Contents/Helpers/headroom")
            return FileManager.default.isExecutableFile(atPath: helper.path) ? helper : nil
        }
    }
#endif
