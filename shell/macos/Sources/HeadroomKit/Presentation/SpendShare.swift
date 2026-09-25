public enum SpendMetric: Sendable, Hashable {
    case cost, tokens

    public static func forList(_ unit: SpendUnit) -> SpendMetric {
        unit == .tokens ? .tokens : .cost
    }

    public static func forRing(_ unit: SpendUnit) -> SpendMetric {
        unit == .cost ? .cost : .tokens
    }

    func basis(costMicros: Int64, tokens: UInt64) -> SpendMetric {
        self == .cost && costMicros <= 0 && tokens > 0 ? .tokens : self
    }

    func value(costMicros: Int64, tokens: UInt64) -> UInt64 {
        switch self {
        case .cost: UInt64(max(0, costMicros))
        case .tokens: tokens
        }
    }
}

public enum SpendShare {
    public static func permille(_ part: UInt64, of total: UInt64) -> Int64 {
        guard total > 0 else { return 0 }
        let scaled = min(part, total).multipliedFullWidth(by: 1000)
        return Int64(total.dividingFullWidth(scaled).quotient)
    }

    public static func fraction(_ part: UInt64, of total: UInt64) -> Double {
        guard total > 0 else { return 0 }
        return min(1, Double(part) / Double(total))
    }

    public static func wholePercent(_ permille: Int64) -> String {
        "\(max(0, permille) / 10)%"
    }

    public static func decimalPercent(_ permille: Int64, language: UILanguage) -> String {
        let value = max(0, permille)
        let separator = language == .ru ? "," : "."
        return "\(value / 10)\(separator)\(value % 10)%"
    }
}
