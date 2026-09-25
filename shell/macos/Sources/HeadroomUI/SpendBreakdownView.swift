#if canImport(AppKit)
    import HeadroomKit
    import SwiftUI

    struct SpendBreakdownView: View {
        let model: SpendBreakdownModel
        let strings: UIStrings
        let select: @MainActor (SpendBreakdown) -> Void
        @Environment(\.headroomReducedMotion) private var reducedMotion

        var body: some View {
            VStack(alignment: .leading, spacing: 4) {
                Rectangle().fill(Palette.separator).frame(height: 1).padding(.trailing, 6)
                HStack(spacing: 8) {
                    if model.showsSwitch {
                        BreakdownSwitch(modes: model.modes, selected: model.mode, strings: strings, select: select)
                    }
                    Spacer(minLength: 0)
                    Text(model.caption).font(Typeface.caption).foregroundStyle(.secondary).monospacedDigit()
                }
                .padding(.top, 4)
                VStack(alignment: .leading, spacing: 0) {
                    ForEach(model.rows) { row in BreakdownRowView(row: row) }
                }
                .id(model.mode)
                .transition(.opacity)
            }
            .animation(Motion.animation(Motion.fast, reduced: reducedMotion), value: model.mode)
        }
    }

    struct BreakdownSwitch: View {
        let modes: [SpendBreakdown]
        let selected: SpendBreakdown
        let strings: UIStrings
        let select: @MainActor (SpendBreakdown) -> Void
        @Namespace private var capsule

        var body: some View {
            HStack(spacing: 2) {
                ForEach(modes) { mode in
                    let isSelected = mode == selected
                    Button {
                        select(mode)
                    } label: {
                        Text(mode.title(strings))
                            .font(isSelected ? Typeface.captionStrong : Typeface.captionMedium)
                            .foregroundStyle(isSelected ? .primary : .secondary)
                            .padding(.vertical, 3)
                            .padding(.horizontal, 10)
                            .background {
                                if isSelected {
                                    Capsule()
                                        .fill(Palette.tray)
                                        .shadow(color: .black.opacity(0.12), radius: 1, y: 1)
                                        .matchedGeometryEffect(id: "breakdown", in: capsule)
                                }
                            }
                            .contentShape(Capsule())
                    }
                    .buttonStyle(.plain)
                    .accessibilityAddTraits(isSelected ? .isSelected : [])
                }
            }
            .padding(2)
            .background(Palette.segmentTrack, in: Capsule())
            .fixedSize()
        }
    }

    struct BreakdownRowView: View {
        let row: BreakdownRow

        var body: some View {
            VStack(alignment: .leading, spacing: 4) {
                HStack(spacing: 6) {
                    icon
                    HStack(spacing: 4) {
                        Text(row.name).font(Typeface.body).lineLimit(1).truncationMode(.middle)
                        if let detail = row.detail {
                            Text(detail).font(Typeface.caption).foregroundStyle(.secondary).lineLimit(1)
                        }
                    }
                    Spacer(minLength: 6)
                    Text(row.share).font(Typeface.caption).foregroundStyle(.secondary)
                    Text(row.value).font(Typeface.bodyMedium).frame(minWidth: 46, alignment: .trailing)
                }
                .monospacedDigit()
                SplitBar(segments: row.segments)
            }
            .padding(EdgeInsets(top: 4, leading: 6, bottom: 5, trailing: 6))
            .contentShape(Rectangle())
            .hoverChip(horizontal: 0, vertical: 0)
        }

        @ViewBuilder
        private var icon: some View {
            switch row.icon {
            case .dot(let color):
                Circle().fill(color.color).frame(width: 8, height: 8)
            case .folder:
                Image(systemName: "folder").font(.system(size: 11)).foregroundStyle(.secondary)
            }
        }
    }

    struct SplitBar: View {
        let segments: [BarSegment]
        @Environment(\.headroomReducedMotion) private var reducedMotion

        var body: some View {
            GeometryReader { proxy in
                ZStack(alignment: .leading) {
                    Capsule().fill(Palette.track)
                    HStack(spacing: 1) {
                        ForEach(segments) { segment in
                            Rectangle()
                                .fill(segment.color.color)
                                .frame(width: max(1, proxy.size.width * CGFloat(segment.fraction)))
                        }
                    }
                    .clipShape(Capsule())
                }
            }
            .frame(height: 3)
            .animation(Motion.animation(Motion.standard, reduced: reducedMotion), value: segments)
            .accessibilityHidden(true)
        }
    }
#endif
