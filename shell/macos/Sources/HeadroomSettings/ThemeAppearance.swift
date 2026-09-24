#if canImport(AppKit)
    import AppKit
    import HeadroomKit

    extension ThemePreference {
        @MainActor
        public var appearance: NSAppearance? {
            switch self {
            case .system: nil
            case .light: NSAppearance(named: .aqua)
            case .dark: NSAppearance(named: .darkAqua)
            }
        }
    }
#endif
