#if canImport(AppKit)
    import HeadroomKit
    import SwiftUI

    struct Pulsing: ViewModifier {
        @State private var dimmed = false
        @Environment(\.headroomReducedMotion) private var reducedMotion

        func body(content: Content) -> some View {
            content
                .opacity(dimmed ? 0.45 : 1)
                .onAppear {
                    guard !reducedMotion else { return }
                    withAnimation(Motion.pulse) { dimmed = true }
                }
        }
    }

    struct SkeletonBlock: View {
        var width: CGFloat?
        var height: CGFloat = 10

        var body: some View {
            RoundedRectangle(cornerRadius: min(3, height / 2), style: .continuous)
                .fill(Palette.skeleton)
                .frame(width: width, height: height)
                .frame(maxWidth: width == nil ? .infinity : nil, alignment: .leading)
        }
    }

    struct SkeletonRows: View {
        let count: Int

        var body: some View {
            VStack(spacing: 0) {
                ForEach(0..<count, id: \.self) { _ in
                    VStack(alignment: .leading, spacing: 6) {
                        HStack {
                            SkeletonBlock(width: 64, height: 11)
                            Spacer()
                            SkeletonBlock(width: 44, height: 9)
                        }
                        SkeletonBlock(height: PopupMetrics.meterHeight)
                        HStack {
                            SkeletonBlock(width: 52, height: 9)
                            Spacer()
                            SkeletonBlock(width: 84, height: 9)
                        }
                    }
                    .padding(.horizontal, PopupMetrics.rowInset)
                    .padding(.vertical, PopupMetrics.barRowPadding)
                }
            }
            .modifier(Pulsing())
            .accessibilityHidden(true)
        }
    }

    struct LoadingSections: View {
        var body: some View {
            VStack(alignment: .leading, spacing: PopupMetrics.sectionGap) {
                ForEach([2, 3], id: \.self) { rows in
                    VStack(alignment: .leading, spacing: PopupMetrics.headerGap) {
                        HStack(spacing: 6) {
                            SkeletonBlock(width: 16, height: 16)
                            SkeletonBlock(width: 96, height: 12)
                        }
                        .padding(.leading, PopupMetrics.headerLeading)
                        .padding(.vertical, 2)
                        .modifier(Pulsing())
                        SkeletonRows(count: rows)
                            .padding(.vertical, PopupMetrics.cardGutter)
                            .cardSurface()
                    }
                }
            }
        }
    }

    struct StatusCard<Actions: View>: View {
        var symbol: String?
        let title: String
        var detail: String?
        @ViewBuilder var actions: () -> Actions

        var body: some View {
            VStack(spacing: 8) {
                if let symbol {
                    Image(systemName: symbol).font(.system(size: 28, weight: .regular)).foregroundStyle(.secondary)
                }
                Text(title).font(Typeface.label)
                if let detail {
                    Text(detail).font(Typeface.body).foregroundStyle(.secondary)
                }
                actions()
            }
            .multilineTextAlignment(.center)
            .fixedSize(horizontal: false, vertical: true)
            .frame(maxWidth: .infinity)
            .padding(.horizontal, 16)
            .padding(.vertical, 24)
            .cardSurface()
        }
    }

    struct PlainActionButton: View {
        let title: String
        let run: @MainActor () -> Void

        var body: some View {
            SmallButton(action: NoticeAction(id: title, title: title, primary: true, busy: false, run: run))
                .padding(.top, 4)
        }
    }

    struct StatusScreen: View {
        let screen: PopupScreen
        let strings: UIStrings
        let retry: @MainActor () -> Void

        var body: some View {
            switch screen {
            case .loading, .dashboard:
                LoadingSections()
            case .incompatible(let text):
                StatusCard(symbol: "exclamationmark.triangle", title: strings.text(text)) { EmptyView() }
            case .serviceDown(let detail):
                StatusCard(
                    symbol: "gauge.with.dots.needle.50percent", title: strings.text(.serviceDownTitle),
                    detail: detail ?? strings.text(.serviceDownDetail)
                ) { EmptyView() }
            case .unreadable(let message):
                StatusCard(title: strings.text(.stateUnreadable), detail: message) {
                    PlainActionButton(title: strings.text(.tryAgain), run: retry)
                }
            case .empty:
                StatusCard(title: strings.text(.noToolsFound)) {
                    PlainActionButton(title: strings.text(.checkAgain), run: retry)
                }
            }
        }
    }
#endif
