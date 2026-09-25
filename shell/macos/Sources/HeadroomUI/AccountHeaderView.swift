#if canImport(AppKit)
    import HeadroomKit
    import SwiftUI

    struct AccountHeaderView: View {
        let header: AccountHeaderModel
        let context: PopupContext
        let now: Timestamp
        let reorder: ReorderHandle?
        let status: StatusNoticeModel?
        let menu: HeaderMenuModel
        let run: @MainActor (HeaderMenuKind, URL?) -> Void
        @State private var hovered = false
        @Environment(\.headroomReducedMotion) private var reducedMotion
        @Environment(\.popupLayout) private var layout

        var body: some View {
            HStack(spacing: 5) {
                ProviderGlyph(provider: header.provider, size: layout.cg.glyphSize)
                titleLine
                statusGlyph
                providerStatusGlyph
                Spacer(minLength: 0)
                HeaderLinkButtons(buttons: menu.buttons, visible: hovered, open: { run(.link($0.kind), $0.url) })
                Image(systemName: "line.3.horizontal")
                    .font(.system(size: 11, weight: .semibold))
                    .foregroundStyle(.tertiary)
                    .opacity(hovered && reorder != nil ? 1 : 0)
                    .accessibilityHidden(true)
            }
            .padding(.leading, PopupMetrics.headerLeading)
            .padding(.trailing, PopupMetrics.headerTrailing)
            .padding(.vertical, 2)
            .contentShape(Rectangle())
            .onHover { hovered = $0 }
            .animation(Motion.animation(Motion.fast, reduced: reducedMotion), value: hovered)
            .gesture(dragGesture, including: reorder == nil ? .subviews : .all)
            .contextMenu { HeaderMenuContent(menu: menu, run: run) }
        }

        private var titleLine: some View {
            HStack(alignment: .firstTextBaseline, spacing: 5) {
                Text(header.title)
                    .font(layout.type.title)
                    .lineLimit(1)
                    .truncationMode(.tail)
                    .layoutPriority(1)
                if let count = header.accountCount {
                    Text(verbatim: "· \(count)").font(layout.type.titleRegular).foregroundStyle(.secondary)
                        .lineLimit(1)
                }
                if let plan = header.plan {
                    Text(plan).font(layout.type.plan).foregroundStyle(.secondary).lineLimit(1).layoutPriority(1)
                }
                if case .outdated(let updatedAt) = header.status {
                    Text(context.strings.text(.outdated))
                        .font(Typeface.caption)
                        .foregroundStyle(.tertiary)
                        .lineLimit(1)
                        .hoverTip(id: "outdated.\(header.title)", text: outdatedTip(updatedAt))
                }
            }
        }

        private var dragGesture: some Gesture {
            DragGesture(minimumDistance: 6, coordinateSpace: .named(PopupSpace.accounts))
                .onChanged { value in reorder?.changed(value) }
                .onEnded { _ in reorder?.ended() }
        }

        @ViewBuilder
        private var statusGlyph: some View {
            switch header.status {
            case .refreshing:
                BusyIndicator(size: 12)
            case .failed(let message):
                Image(systemName: "exclamationmark.triangle.fill")
                    .font(.system(size: 10, weight: .semibold))
                    .foregroundStyle(Palette.notice)
                    .hoverTip(
                        id: "failed.\(header.title)",
                        text: message.isEmpty ? context.strings.text(.refreshFailed) : message)
            case .outdated, .none:
                EmptyView()
            }
        }

        @ViewBuilder
        private var providerStatusGlyph: some View {
            if let status {
                Image(systemName: status.kind == .error ? "minus.circle.fill" : "exclamationmark.triangle.fill")
                    .font(.system(size: 10, weight: .semibold))
                    .foregroundStyle(status.kind == .error ? Palette.crit : Palette.notice)
                    .hoverTip(id: "provider-status.\(header.provider)", text: status.title)
                    .transition(.opacity)
            }
        }

        private func outdatedTip(_ updatedAt: Timestamp?) -> String? {
            guard let updatedAt else { return nil }
            return context.strings.fill(.lastUpdated, ["ago": context.formatter.agoText(updatedAt, now: now)])
        }
    }

    struct HeaderLinkButtons: View {
        let buttons: [HeaderLinkButton]
        let visible: Bool
        let open: @MainActor (HeaderLinkButton) -> Void

        var body: some View {
            HStack(spacing: 2) {
                ForEach(buttons) { button in
                    Button {
                        open(button)
                    } label: {
                        Image(systemName: HeaderSymbols.link(button.kind))
                            .font(.system(size: 11, weight: .medium))
                            .foregroundStyle(.secondary)
                            .frame(width: 22, height: 22)
                    }
                    .buttonStyle(TintButtonStyle(circle: true))
                    .hoverTip(id: "link.\(button.url.absoluteString)", text: button.tip)
                    .accessibilityLabel(button.tip)
                }
            }
            .opacity(visible ? 1 : 0)
            .allowsHitTesting(visible)
        }
    }

    enum HeaderSymbols {
        static func link(_ kind: ProviderLinkKind) -> String {
            switch kind {
            case .status: "waveform.path.ecg"
            case .usage: "chart.bar"
            case .dashboard: "arrow.up.right.square"
            }
        }

        static func item(_ kind: HeaderMenuKind) -> String {
            switch kind {
            case .refresh: "arrow.clockwise"
            case .hide: "eye.slash"
            case .star: "star"
            case .link(let kind): Self.link(kind)
            case .share: "square.and.arrow.up"
            case .copyText: "doc.on.doc"
            }
        }
    }

    struct HeaderMenuContent: View {
        let menu: HeaderMenuModel
        let run: @MainActor (HeaderMenuKind, URL?) -> Void

        var body: some View {
            ForEach(Array(menu.groups.enumerated()), id: \.offset) { index, group in
                if index > 0 { Divider() }
                ForEach(group) { item in entry(item) }
            }
        }

        @ViewBuilder
        private func entry(_ item: HeaderMenuItem) -> some View {
            if item.kind == .star {
                Toggle(isOn: Binding(get: { item.checked }, set: { _ in run(.star, nil) })) {
                    Label(item.title, systemImage: HeaderSymbols.item(item.kind))
                }
            } else {
                Button {
                    run(item.kind, item.url)
                } label: {
                    Label(item.title, systemImage: HeaderSymbols.item(item.kind))
                }
            }
        }
    }
#endif
