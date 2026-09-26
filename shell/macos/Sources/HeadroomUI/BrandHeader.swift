#if canImport(AppKit)
    import HeadroomKit
    import SwiftUI

    struct BrandHeader<Trailing: View>: View {
        static var markSize: CGFloat { 16 }

        @ViewBuilder let trailing: () -> Trailing
        @Environment(\.popupLayout) private var layout

        var body: some View {
            HStack(spacing: 6) {
                BrandMarkView(size: Self.markSize)
                    .foregroundStyle(.secondary)
                Text(verbatim: BrandMark.name)
                    .font(layout.type.title)
                    .lineLimit(1)
                Spacer(minLength: 0)
                trailing()
            }
            .padding(.leading, PopupMetrics.headerLeading)
            .padding(.trailing, PopupMetrics.headerTrailing)
            .frame(minHeight: PopupMetrics.refreshSize)
        }
    }
#endif
