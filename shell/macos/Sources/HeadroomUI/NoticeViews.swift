#if canImport(AppKit)
    import HeadroomKit
    import SwiftUI

    struct NoticeAction: Identifiable {
        let id: String
        let title: String
        let primary: Bool
        let busy: Bool
        let run: @MainActor () -> Void
    }

    struct NoticePlate: View {
        let kind: NoticeKind
        let title: String
        let detail: String?
        let note: String?
        let actions: [NoticeAction]

        var body: some View {
            HStack(alignment: actions.count == 1 ? .center : .top, spacing: 8) {
                NoticeTile(kind: kind)
                VStack(alignment: .leading, spacing: 1) {
                    Text(title).font(Typeface.captionStrong)
                    if let detail { Text(detail).font(Typeface.caption2).foregroundStyle(.secondary) }
                    if let note { Text(note).font(Typeface.caption2).foregroundStyle(.secondary).padding(.top, 3) }
                    if actions.count > 1 { actionRow.padding(.top, 6) }
                }
                .fixedSize(horizontal: false, vertical: true)
                .frame(maxWidth: .infinity, alignment: .leading)
                if actions.count == 1 { actionRow }
            }
            .padding(.horizontal, 9)
            .padding(.vertical, 8)
            .background(background, in: shape)
            .overlay(shape.strokeBorder(border, lineWidth: 1))
            .padding(.horizontal, 8)
            .padding(.vertical, 6)
        }

        private var shape: RoundedRectangle {
            RoundedRectangle(cornerRadius: PopupMetrics.noticeRadius, style: .continuous)
        }

        private var actionRow: some View {
            HStack(spacing: 4) {
                ForEach(actions) { action in SmallButton(action: action) }
            }
        }

        private var background: Color {
            kind == .error ? Palette.errorBackground : Palette.noticeBackground
        }

        private var border: Color {
            kind == .error ? Palette.errorBorder : Palette.noticeBorder
        }
    }

    struct NoticeTile: View {
        let kind: NoticeKind

        var body: some View {
            Image(systemName: symbol)
                .font(.system(size: 12, weight: .semibold))
                .foregroundStyle(kind == .error ? Palette.errorForeground : Palette.noticeForeground)
                .frame(width: 24, height: 24)
                .background(
                    kind == .error ? Palette.errorTile : Palette.noticeTile,
                    in: RoundedRectangle(cornerRadius: 7, style: .continuous))
        }

        private var symbol: String {
            switch kind {
            case .error: "xmark.octagon.fill"
            case .signIn: "person.crop.circle"
            case .warning, .info: "exclamationmark.triangle.fill"
            }
        }
    }

    struct NoticeLine: View {
        let text: String

        var body: some View {
            HStack(alignment: .firstTextBaseline, spacing: 6) {
                Image(systemName: "info.circle").font(.system(size: 11))
                Text(text).font(Typeface.caption).fixedSize(horizontal: false, vertical: true)
            }
            .foregroundStyle(.secondary)
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.horizontal, PopupMetrics.rowInset)
            .padding(.top, 6)
        }
    }

    struct SmallButton: View {
        let action: NoticeAction

        var body: some View {
            Button {
                action.run()
            } label: {
                HStack(spacing: 4) {
                    if action.busy { BusyIndicator(size: 10) }
                    Text(action.title).font(Typeface.captionStrong).lineLimit(1)
                }
                .foregroundStyle(action.primary ? Color.white : Color.primary)
                .padding(.horizontal, 7)
                .padding(.vertical, 3)
                .background {
                    if action.primary {
                        RoundedRectangle(cornerRadius: 5, style: .continuous).fill(Palette.ok)
                    } else {
                        RoundedRectangle(cornerRadius: 5, style: .continuous).strokeBorder(
                            Palette.separator, lineWidth: 1)
                    }
                }
            }
            .buttonStyle(TintButtonStyle(cornerRadius: 5))
            .disabled(action.busy)
            .fixedSize()
        }
    }

    struct BusyIndicator: View {
        let size: CGFloat
        @Environment(\.headroomReducedMotion) private var reducedMotion

        var body: some View {
            Group {
                if reducedMotion {
                    Image(systemName: "arrow.triangle.2.circlepath").font(.system(size: size * 0.9))
                } else {
                    ProgressView().progressViewStyle(.circular).controlSize(.mini)
                }
            }
            .frame(width: size, height: size)
        }
    }
#endif
