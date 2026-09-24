import Foundation

extension DisplayFormatter {
    private static let microsPerCent: Int64 = 10_000
    private static let abbreviateMoneyFromCents: Int64 = 100_000
    private static let compactWordsFrom: UInt64 = 1000
    private static let currencySymbols = ["USD": "$", "CNY": "¥", "EUR": "€"]
    private static let units: [(size: Double, suffix: String, russian: String)] = [
        (1e9, "B", "\u{00A0}млрд"),
        (1e6, "M", "\u{00A0}млн"),
        (1e3, "K", "\u{00A0}тыс."),
    ]

    public func exactUSD(micros: Int64) -> String {
        money(currency: "USD", micros: micros)
    }

    public func money(currency: String, micros: Int64) -> String {
        let cents = Self.cents(of: micros)
        let magnitude = cents.magnitude
        let rest = magnitude % 100
        let sign = cents < 0 ? "-" : ""
        let digits = "\(Self.grouped(magnitude / 100, separator: ",")).\(rest < 10 ? "0" : "")\(rest)"
        guard let symbol = Self.currencySymbols[currency] else { return "\(sign)\(digits) \(currency)" }
        return "\(sign)\(symbol)\(digits)"
    }

    public func usd(micros: Int64) -> String {
        let cents = Self.cents(of: micros)
        guard cents.magnitude >= Self.abbreviateMoneyFromCents.magnitude else { return exactUSD(micros: micros) }
        let (text, suffix, _) = Self.abbreviate(Double(cents.magnitude) / 100, digits: Self.moneyDigits)
        return "\(cents < 0 ? "-" : "")$\(text)\(suffix)"
    }

    public func exactTokens(_ count: UInt64) -> String {
        Self.grouped(count, separator: language == .ru ? "\u{00A0}" : ",")
    }

    public func compactTokens(_ count: UInt64) -> String {
        let (text, suffix, russian) = Self.abbreviate(Double(count), digits: Self.tokenDigits)
        guard language == .ru else { return "\(text)\(suffix)" }
        return text.replacingOccurrences(of: ".", with: ",") + russian
    }

    public func exactTokensText(_ count: UInt64) -> String {
        "\(exactTokens(count)) \(strings.tokenWord(count))"
    }

    public func compactTokensText(_ count: UInt64) -> String {
        let wordCount = count < Self.compactWordsFrom ? count : 0
        return "\(compactTokens(count)) \(strings.tokenWord(wordCount))"
    }

    static func cents(of micros: Int64) -> Int64 {
        let magnitude = micros.magnitude
        let step = microsPerCent.magnitude
        let rounded = Int64(magnitude / step + (magnitude % step >= step / 2 ? 1 : 0))
        return micros < 0 ? -rounded : rounded
    }

    static func grouped(_ value: UInt64, separator: String) -> String {
        let digits = Array(String(value))
        var result = ""
        for (index, digit) in digits.enumerated() {
            if index > 0 && (digits.count - index) % 3 == 0 { result += separator }
            result.append(digit)
        }
        return result
    }

    private static func tokenDigits(_ scaled: Double) -> Int { scaled >= 100 ? 0 : 1 }

    private static func moneyDigits(_ scaled: Double) -> Int { scaled >= 100 ? 0 : scaled >= 10 ? 1 : 2 }

    private static func abbreviate(
        _ value: Double, digits: (Double) -> Int
    ) -> (text: String, suffix: String, russian: String) {
        guard let unit = units.first(where: { value.magnitude >= $0.size }) else {
            return (String(format: "%.0f", value), "", "")
        }
        let scaled = value / unit.size
        let text = String(format: "%.\(digits(scaled.magnitude))f", scaled)
        return (trimmingZeroFraction(text), unit.suffix, unit.russian)
    }

    private static func trimmingZeroFraction(_ text: String) -> String {
        guard let dot = text.firstIndex(of: "."), text[text.index(after: dot)...].allSatisfy({ $0 == "0" }) else {
            return text
        }
        return String(text[..<dot])
    }
}
