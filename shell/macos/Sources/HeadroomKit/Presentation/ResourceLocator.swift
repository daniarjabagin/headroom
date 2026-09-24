import Foundation

public enum ResourceLocator {
    public static let uiBundleName = "HeadroomMac_HeadroomUI.bundle"

    public static func directory(named folder: String, inBundle bundleName: String, roots: [URL]) -> URL? {
        candidates(folder: folder, bundleName: bundleName, roots: roots).first { isDirectory($0) }
    }

    static func candidates(folder: String, bundleName: String, roots: [URL]) -> [URL] {
        roots.flatMap { root -> [URL] in
            let bundle = root.appendingPathComponent(bundleName, isDirectory: true)
            return [
                bundle.appendingPathComponent("Contents/Resources", isDirectory: true)
                    .appendingPathComponent(folder, isDirectory: true),
                bundle.appendingPathComponent(folder, isDirectory: true),
            ]
        }
    }

    private static func isDirectory(_ url: URL) -> Bool {
        var directory: ObjCBool = false
        return FileManager.default.fileExists(atPath: url.path, isDirectory: &directory) && directory.boolValue
    }
}
