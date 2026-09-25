#if canImport(AppKit)
    import HeadroomKit
    import SwiftUI

    struct StatusNoticeView: View {
        let notice: StatusNoticeModel
        let open: @MainActor (URL) -> Void
        @Environment(\.popupLayout) private var layout

        var body: some View {
            Group {
                if layout.isCompact {
                    line
                } else {
                    plate
                }
            }
            .transition(.opacity.combined(with: .offset(y: 4)))
        }

        private var isError: Bool { notice.kind == .error }

        private var plate: some View {
            HStack(alignment: .top, spacing: 8) {
                NoticeTile(kind: notice.kind)
                VStack(alignment: .leading, spacing: 1) {
                    Text(notice.title).font(Typeface.captionStrong).fixedSize(horizontal: false, vertical: true)
                    if let detail = notice.detail {
                        Text(detail).font(Typeface.caption2).foregroundStyle(.secondary)
                    }
                    HStack(spacing: 5) {
                        if let started = notice.started {
                            Text(started)
                                .font(Typeface.caption2)
                                .foregroundStyle(.secondary)
                                .hoverTip(id: "\(notice.id).started", text: notice.startedTip)
                        }
                        Spacer(minLength: 0)
                        link
                    }
                    .padding(.top, 3)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .padding(.horizontal, 9)
            .padding(.vertical, 8)
            .background(isError ? Palette.errorBackground : Palette.noticeBackground, in: shape)
            .overlay(shape.strokeBorder(isError ? Palette.errorBorder : Palette.noticeBorder, lineWidth: 1))
            .padding(.horizontal, 8)
            .padding(.vertical, 6)
        }

        private var line: some View {
            HStack(spacing: 7) {
                Circle().fill(isError ? Palette.crit : Palette.notice).frame(width: 7, height: 7)
                Text(notice.heading).font(Typeface.captionStrong).lineLimit(1)
                if let elapsed = notice.elapsed {
                    Text(elapsed)
                        .font(Typeface.caption)
                        .foregroundStyle(.secondary)
                        .hoverTip(id: "\(notice.id).elapsed", text: notice.startedTip)
                }
                Spacer(minLength: 0)
                link
            }
            .monospacedDigit()
            .hoverTip(id: "\(notice.id).line", text: notice.title)
            .padding(.horizontal, PopupMetrics.rowInset)
            .padding(.top, 6)
            .padding(.bottom, 2)
        }

        @ViewBuilder
        private var link: some View {
            if let url = notice.url {
                Button {
                    open(url)
                } label: {
                    HStack(spacing: 2) {
                        Text(notice.linkTitle)
                        Image(systemName: "arrow.up.right.square").font(.system(size: 9, weight: .semibold))
                    }
                    .font(Typeface.captionStrong)
                    .foregroundStyle(Palette.ok)
                }
                .buttonStyle(.plain)
                .hoverChip(horizontal: 3, vertical: 1)
            }
        }

        private var shape: RoundedRectangle {
            RoundedRectangle(cornerRadius: PopupMetrics.noticeRadius, style: .continuous)
        }
    }
#endif
