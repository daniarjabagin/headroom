#if canImport(AppKit)
    import AppKit
    import HeadroomKit
    import SwiftUI

    extension Tone {
        public var color: Color {
            switch self {
            case .good: .accentColor
            case .warning: Color(nsColor: .systemYellow)
            case .critical: Color(nsColor: .systemRed)
            case .neutral: Color(nsColor: .systemGray)
            }
        }
    }

    enum PopupMetrics {
        static let width: CGFloat = 320
        static let cornerRadius: CGFloat = 13
        static let cardRadius: CGFloat = 12
        static let padding: CGFloat = 14
        static let sectionGap: CGFloat = 14
        static let meterHeight: CGFloat = 5
        static let tickWidth: CGFloat = 2
        static let tickHeight: CGFloat = 9
    }
#endif
