#if canImport(AppKit)
    import HeadroomKit
    import SwiftUI

    struct ReorderHandle {
        let changed: @MainActor (DragGesture.Value) -> Void
        let ended: @MainActor () -> Void
    }

    struct DragState: Equatable {
        let id: String
        var translation: CGFloat
        var pointer: CGFloat
    }

    struct AccountList: View {
        let sections: [AccountSectionModel]
        let context: PopupContext
        let now: Timestamp
        let expanded: Set<String>
        let toggleExpanded: @MainActor (String) -> Void
        let reorder: @MainActor ([String]) -> Void
        @State private var drag: DragState?
        @State private var frames: [String: CGRect] = [:]
        @Environment(\.headroomReducedMotion) private var reducedMotion
        @Environment(\.popupLayout) private var layout

        var body: some View {
            VStack(alignment: .leading, spacing: layout.cg.sectionGap) {
                ForEach(sections) { section in
                    sectionView(section)
                }
            }
            .overlay(alignment: .topLeading) { dropIndicator }
            .coordinateSpace(.named(PopupSpace.accounts))
            .animation(Motion.animation(Motion.standard, reduced: reducedMotion), value: sections.map(\.id))
        }

        private var canReorder: Bool { sections.count > 1 }

        private func sectionView(_ section: AccountSectionModel) -> some View {
            let lifted = drag?.id == section.id
            return AccountSectionView(
                section: section, context: context, now: now, expanded: expanded.contains(section.id),
                reorder: canReorder ? handle(for: section.id) : nil, toggleExpanded: { toggleExpanded(section.id) }
            )
            .background {
                GeometryReader { proxy in
                    Color.clear.onChange(of: proxy.frame(in: .named(PopupSpace.accounts)), initial: true) { _, frame in
                        frames[section.id] = frame
                    }
                }
            }
            .background {
                if lifted {
                    RoundedRectangle(cornerRadius: PopupMetrics.cardRadius, style: .continuous)
                        .fill(Palette.tray)
                        .shadow(color: .black.opacity(0.18), radius: 14, y: 8)
                }
            }
            .scaleEffect(lifted ? 1.018 : 1)
            .offset(y: lifted ? drag?.translation ?? 0 : 0)
            .zIndex(lifted ? 1 : 0)
        }

        private func handle(for id: String) -> ReorderHandle {
            ReorderHandle(changed: { value in dragChanged(id, value) }, ended: { finishDrag() })
        }

        private func dragChanged(_ id: String, _ value: DragGesture.Value) {
            let next = DragState(id: id, translation: value.translation.height, pointer: value.location.y)
            guard drag != nil else {
                Motion.perform(Motion.fast, reduced: reducedMotion) { drag = next }
                return
            }
            drag = next
        }

        @ViewBuilder
        private var dropIndicator: some View {
            if let drag, let position = indicatorPosition(drag) {
                Capsule()
                    .fill(Palette.ok)
                    .frame(height: 2)
                    .offset(y: position - 1)
                    .allowsHitTesting(false)
            }
        }

        private func others(excluding id: String) -> [SectionSpan] {
            sections.filter { $0.id != id }.compactMap { frames[$0.id] }.map {
                SectionSpan(top: Double($0.minY), bottom: Double($0.maxY))
            }
        }

        private func target(for drag: DragState) -> (from: Int, to: Int)? {
            guard let from = sections.firstIndex(where: { $0.id == drag.id }) else { return nil }
            return (from, AccountOrder.dropIndex(others: others(excluding: drag.id), pointer: Double(drag.pointer)))
        }

        private func indicatorPosition(_ drag: DragState) -> CGFloat? {
            guard let move = target(for: drag) else { return nil }
            return AccountOrder.indicatorPosition(
                others: others(excluding: drag.id), target: move.to, from: move.from,
                gap: layout.sectionGap
            ).map { CGFloat($0) }
        }

        private func finishDrag() {
            guard let current = drag else { return }
            let move = target(for: current)
            Motion.perform(Motion.standard, reduced: reducedMotion) { drag = nil }
            guard let move, move.from != move.to else { return }
            reorder(AccountOrder.moveItem(sections, from: move.from, to: move.to).flatMap(\.memberIDs))
        }
    }
#endif
