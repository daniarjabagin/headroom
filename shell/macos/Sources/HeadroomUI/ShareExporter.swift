#if canImport(AppKit)
    import AppKit
    import Foundation
    import HeadroomKit
    import SwiftUI

    enum ShareOutcome {
        case copied(saved: Bool)
        case failed
    }

    @MainActor
    enum ShareExporter {
        static let folder = "Headroom"

        static func export(_ card: ShareCardModel, icons: ProviderIconStore) -> ShareOutcome {
            guard let png = render(card, icons: icons) else { return .failed }
            let pasteboard = NSPasteboard.general
            pasteboard.clearContents()
            guard pasteboard.setData(png, forType: .png) else { return .failed }
            return .copied(saved: save(png, name: card.fileName))
        }

        static func copy(text: String) {
            NSPasteboard.general.clearContents()
            NSPasteboard.general.setString(text, forType: .string)
        }

        private static func render(_ card: ShareCardModel, icons: ProviderIconStore) -> Data? {
            let dark = NSApp.effectiveAppearance.bestMatch(from: [.aqua, .darkAqua]) == .darkAqua
            let view = ShareCardView(card: card, style: ShareCardStyle.make(dark: dark))
                .environment(icons)
                .environment(\.colorScheme, dark ? .dark : .light)
            let renderer = ImageRenderer(content: view)
            renderer.scale = CGFloat(ShareCardStyle.scale)
            guard let image = renderer.cgImage else { return nil }
            return NSBitmapImageRep(cgImage: image).representation(using: .png, properties: [:])
        }

        private static func save(_ png: Data, name: String) -> Bool {
            guard let pictures = FileManager.default.urls(for: .picturesDirectory, in: .userDomainMask).first else {
                return false
            }
            let directory = pictures.appendingPathComponent(folder, isDirectory: true)
            do {
                try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
                try png.write(to: directory.appendingPathComponent(name), options: .atomic)
                return true
            } catch {
                return false
            }
        }
    }
#endif
