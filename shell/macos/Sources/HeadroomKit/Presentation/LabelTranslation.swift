import Foundation

public enum LabelTranslation {
    private static let russian: [String: String] = [
        "Credits": "Кредиты",
        "Extra usage": "Доп. использование",
        "Balance": "Баланс",
        "Vouchers": "Ваучеры",
        "Cash": "Денежный баланс",
        "Credit balance": "Кредиты",
        "Organization credits": "Кредиты организации",
        "Point balance": "Баланс баллов",
        "Bonus credits": "Бонусные кредиты",
        "Monthly credits": "Кредиты на месяц",
    ]

    public static func translate(_ label: String, language: UILanguage) -> String {
        guard language == .ru else { return label }
        return russian[label] ?? label
    }
}
