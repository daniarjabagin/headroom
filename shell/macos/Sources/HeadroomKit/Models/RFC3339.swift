import Foundation

enum RFC3339 {
    private static let secondsPerDay: Int64 = 86_400

    static func parse(_ text: String) -> Date? {
        var scanner = ByteScanner(bytes: Array(text.utf8))
        guard let civil = scanner.civilDate(), scanner.consume(anyOf: "Tt "),
            let clock = scanner.clockTime()
        else { return nil }
        let fraction = scanner.fraction()
        guard let offset = scanner.utcOffset(), scanner.isAtEnd, civil.isValid, clock.isValid else {
            return nil
        }
        let seconds = civil.daysSinceEpoch * secondsPerDay + clock.seconds - offset
        return Date(timeIntervalSince1970: Double(seconds) + fraction)
    }
}

private struct CivilDate {
    let year: Int64
    let month: Int64
    let day: Int64

    var isValid: Bool { (1...12).contains(month) && (1...31).contains(day) }

    var daysSinceEpoch: Int64 {
        let shiftedYear = month <= 2 ? year - 1 : year
        let era = (shiftedYear >= 0 ? shiftedYear : shiftedYear - 399) / 400
        let yearOfEra = shiftedYear - era * 400
        let shiftedMonth = month > 2 ? month - 3 : month + 9
        let dayOfYear = (153 * shiftedMonth + 2) / 5 + day - 1
        let dayOfEra = yearOfEra * 365 + yearOfEra / 4 - yearOfEra / 100 + dayOfYear
        return era * 146_097 + dayOfEra - 719_468
    }
}

private struct ClockTime {
    let hour: Int64
    let minute: Int64
    let second: Int64

    var isValid: Bool { hour < 24 && minute < 60 && second <= 60 }
    var seconds: Int64 { hour * 3600 + minute * 60 + second }
}

private struct ByteScanner {
    let bytes: [UInt8]
    var index = 0

    var isAtEnd: Bool { index == bytes.count }

    mutating func civilDate() -> CivilDate? {
        guard let year = digits(4), consume(anyOf: "-"), let month = digits(2), consume(anyOf: "-"),
            let day = digits(2)
        else { return nil }
        return CivilDate(year: year, month: month, day: day)
    }

    mutating func clockTime() -> ClockTime? {
        guard let hour = digits(2), consume(anyOf: ":"), let minute = digits(2), consume(anyOf: ":"),
            let second = digits(2)
        else { return nil }
        return ClockTime(hour: hour, minute: minute, second: second)
    }

    mutating func fraction() -> Double {
        guard consume(anyOf: ".") else { return 0 }
        var value = 0.0
        var scale = 0.1
        while let digit = peekDigit() {
            value += Double(digit) * scale
            scale /= 10
            index += 1
        }
        return value
    }

    mutating func utcOffset() -> Int64? {
        if consume(anyOf: "Zz") { return 0 }
        guard index < bytes.count else { return nil }
        let sign: Int64 = bytes[index] == UInt8(ascii: "-") ? -1 : 1
        guard consume(anyOf: "+-"), let hours = digits(2), consume(anyOf: ":"), let minutes = digits(2)
        else { return nil }
        return sign * (hours * 3600 + minutes * 60)
    }

    mutating func consume(anyOf characters: String) -> Bool {
        guard index < bytes.count, characters.utf8.contains(bytes[index]) else { return false }
        index += 1
        return true
    }

    private mutating func digits(_ count: Int) -> Int64? {
        var value: Int64 = 0
        for _ in 0..<count {
            guard let digit = peekDigit() else { return nil }
            value = value * 10 + digit
            index += 1
        }
        return value
    }

    private func peekDigit() -> Int64? {
        guard index < bytes.count else { return nil }
        let byte = bytes[index]
        guard byte >= UInt8(ascii: "0"), byte <= UInt8(ascii: "9") else { return nil }
        return Int64(byte - UInt8(ascii: "0"))
    }
}
