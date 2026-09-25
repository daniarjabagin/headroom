import Foundation

extension TimeOfDay {
    public init(date: Date, calendar: Calendar) {
        let parts = calendar.dateComponents([.hour, .minute], from: date)
        self = TimeOfDay.at(parts.hour ?? 0, parts.minute ?? 0)
    }

    public func date(on day: Date, calendar: Calendar) -> Date {
        calendar.date(bySettingHour: hour, minute: minute, second: 0, of: day) ?? calendar.startOfDay(for: day)
    }
}

public struct AlertProvider: Sendable, Hashable, Identifiable {
    public let id: String
    public let name: String
}

extension SettingsOptions {
    public static func alertProviders(_ accounts: [Account]) -> [AlertProvider] {
        var seen: Set<String> = []
        return accounts.compactMap { account in
            guard seen.insert(account.provider).inserted else { return nil }
            return AlertProvider(id: account.provider, name: account.providerName)
        }
    }
}
