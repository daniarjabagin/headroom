#if canImport(AppKit)
    import HeadroomKit
    import SwiftUI

    struct ShareCardView: View {
        static let maxRows = 3

        let card: ShareCardModel
        let style: ShareCardStyle

        private var foreground: Color { ShareColor.make(style.foreground) }
        private var dim: Color { foreground.opacity(ShareCardStyle.dimAlpha) }
        private var line: Color { foreground.opacity(ShareCardStyle.lineAlpha) }

        var body: some View {
            VStack(alignment: .leading, spacing: 0) {
                topBar
                Rectangle().fill(line).frame(height: 1)
                Spacer(minLength: 0)
                HStack(alignment: .center, spacing: 24) {
                    hero
                    Spacer(minLength: 0)
                    limitsCard
                }
                Spacer(minLength: 0)
                Rectangle().fill(line).frame(height: 1)
                footer
            }
            .padding(EdgeInsets(top: 22, leading: 28, bottom: 18, trailing: 28))
            .frame(width: CGFloat(ShareCardStyle.logicalWidth), height: CGFloat(ShareCardStyle.logicalHeight))
            .background(ShareColor.make(style.background))
            .foregroundStyle(foreground)
            .monospacedDigit()
        }

        private var topBar: some View {
            HStack(spacing: 6) {
                BrandMarkView(size: 20)
                Text(verbatim: ShareCardModel.wordmark).font(.system(size: 16, weight: .semibold))
                Spacer(minLength: 0)
                Text(card.stamp).font(.system(size: 10, weight: .medium)).tracking(0.8).foregroundStyle(dim)
            }
            .padding(.bottom, 12)
        }

        private var hero: some View {
            VStack(alignment: .leading, spacing: 6) {
                HStack(spacing: 6) {
                    ProviderGlyph(provider: card.provider, size: 16)
                    Text(card.providerName).font(.system(size: 13, weight: .semibold))
                    if let subtitle = card.subtitle { Text(subtitle).font(.system(size: 13)).foregroundStyle(dim) }
                }
                .lineLimit(1)
                HStack(alignment: .firstTextBaseline, spacing: 6) {
                    Text(card.hero).font(.system(size: 58, weight: .bold)).tracking(-1.7)
                    Text(card.heroWord).font(.system(size: 22, weight: .semibold))
                }
                .lineLimit(1)
                .minimumScaleFactor(0.6)
                Text(card.heroDetail).font(.system(size: 12)).foregroundStyle(dim).lineLimit(1)
                HStack(spacing: 3) {
                    ForEach(Array(card.heroMeters.enumerated()), id: \.offset) { _, fill in
                        ShareBar(fill: fill, height: 7, track: line, color: foreground)
                    }
                }
                .frame(width: 250, height: 7)
            }
        }

        private var limitsCard: some View {
            VStack(alignment: .leading, spacing: 12) {
                ForEach(card.rows.prefix(Self.maxRows)) { row in
                    VStack(alignment: .leading, spacing: 4) {
                        Text(row.label).font(.system(size: 13, weight: .semibold))
                        HStack(spacing: 4) {
                            ForEach(row.segments) { segment in
                                ShareBar(
                                    fill: segment.fill, height: 5, track: ShareColor.make(style.track),
                                    color: ShareColor.make(style.tone(segment.tone)), tick: segment.tick,
                                    tickColor: dim)
                            }
                        }
                        .frame(height: 9)
                        HStack {
                            Text(row.headline)
                            Spacer(minLength: 6)
                            Text(row.trailing).foregroundStyle(dim)
                        }
                        .font(.system(size: 12))
                        .lineLimit(1)
                    }
                }
            }
            .padding(14)
            .frame(width: 262, alignment: .leading)
            .background(ShareColor.make(style.card), in: RoundedRectangle(cornerRadius: 12, style: .continuous))
        }

        private var footer: some View {
            HStack(spacing: 6) {
                Rectangle()
                    .fill(ShareColor.make(ShareCardStyle.accent))
                    .frame(width: 8, height: 8)
                    .transformEffect(CGAffineTransform(a: 1, b: 0, c: -0.29, d: 1, tx: 2, ty: 0))
                Text(card.tagline)
                Spacer(minLength: 0)
                Text(verbatim: ShareCardModel.wordmark)
            }
            .font(.system(size: 10))
            .foregroundStyle(dim)
            .padding(.top, 10)
        }
    }

    struct ShareBar: View {
        let fill: Double
        let height: CGFloat
        let track: Color
        let color: Color
        var tick: Double?
        var tickColor = Color.clear

        var body: some View {
            GeometryReader { proxy in
                let width = proxy.size.width
                ZStack(alignment: .leading) {
                    Capsule().fill(track).frame(height: height)
                    Capsule().fill(color).frame(
                        width: max(fill > 0 ? height : 0, width * CGFloat(min(1, max(0, fill)))), height: height)
                    if let tick {
                        RoundedRectangle(cornerRadius: 1)
                            .fill(tickColor)
                            .frame(width: 2, height: height + 4)
                            .offset(x: min(max(0, width * CGFloat(min(1, max(0, tick))) - 1), max(0, width - 2)))
                    }
                }
                .frame(height: proxy.size.height)
            }
        }
    }

    enum ShareColor {
        static func make(_ hex: UInt32) -> Color {
            Color(
                .sRGB, red: Double((hex >> 16) & 0xFF) / 255, green: Double((hex >> 8) & 0xFF) / 255,
                blue: Double(hex & 0xFF) / 255)
        }
    }
#endif
