import Foundation

public enum LogFile {
    public static let rotationSize: UInt64 = 5 * 1024 * 1024

    public static func daemonLog(home: URL) -> URL {
        home.appendingPathComponent("Library/Logs/Headroom/daemon.log")
    }

    static func openForAppending(_ url: URL) throws -> FileHandle {
        let manager = FileManager.default
        try manager.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        try rotateIfLarge(url, manager: manager)
        if !manager.fileExists(atPath: url.path) {
            _ = manager.createFile(atPath: url.path, contents: nil, attributes: [.posixPermissions: 0o600])
        }
        let handle = try FileHandle(forWritingTo: url)
        try handle.seekToEnd()
        return handle
    }

    private static func rotateIfLarge(_ url: URL, manager: FileManager) throws {
        let attributes = try? manager.attributesOfItem(atPath: url.path)
        let size = (attributes?[.size] as? NSNumber)?.uint64Value ?? 0
        guard size > rotationSize else { return }
        let previous = url.appendingPathExtension("1")
        try? manager.removeItem(at: previous)
        try manager.moveItem(at: url, to: previous)
    }
}
