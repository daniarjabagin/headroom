public struct SectionSpan: Sendable, Hashable {
    public let top: Double
    public let bottom: Double

    public init(top: Double, bottom: Double) {
        self.top = top
        self.bottom = bottom
    }

    var middle: Double { (top + bottom) / 2 }
}

public enum AccountOrder {
    public static func moveItem<Item>(_ items: [Item], from: Int, to: Int) -> [Item] {
        guard from != to, items.indices.contains(from) else { return items }
        var moved = items
        let item = moved.remove(at: from)
        moved.insert(item, at: min(max(to, 0), moved.count))
        return moved
    }

    public static func mergeOrder(all: [String], visible: [String]) -> [String] {
        let visibleSet = Set(visible)
        var queue = visible[...]
        return all.map { id in
            guard visibleSet.contains(id), let next = queue.popFirst() else { return id }
            return next
        }
    }

    public static func apply<Item: Identifiable>(_ order: [String]?, to items: [Item]) -> [Item] where Item.ID == String {
        guard let order else { return items }
        let rank = Dictionary(order.enumerated().map { ($1, $0) }, uniquingKeysWith: { first, _ in first })
        return items.enumerated()
            .sorted { lhs, rhs in
                let left = rank[lhs.element.id] ?? order.count + lhs.offset
                let right = rank[rhs.element.id] ?? order.count + rhs.offset
                return left < right
            }
            .map(\.element)
    }

    public static func reconcile(local: [String]?, incoming: [String]) -> [String]? {
        guard let local, local != incoming, Set(local) == Set(incoming) else { return nil }
        return local
    }

    public static func dropIndex(others: [SectionSpan], pointer: Double) -> Int {
        others.filter { $0.middle < pointer }.count
    }

    public static func indicatorPosition(others: [SectionSpan], target: Int, from: Int, gap: Double) -> Double? {
        guard let first = others.first, target != from else { return nil }
        guard target > 0 else { return first.top - gap / 2 }
        return others[min(target, others.count) - 1].bottom + gap / 2
    }
}
