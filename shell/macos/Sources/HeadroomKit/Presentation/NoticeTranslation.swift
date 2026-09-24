import Foundation

public enum NoticeTranslation {
    private static let russianFixed: [String: String] = [
        "Weekly limit shared with Codex Cloud": "Недельный лимит общий с Codex Cloud",
        "Offline — showing limits from local logs.": "Нет сети — лимиты по локальным журналам.",
        "Sign-in expired — open Codex to sign in again. Showing limits from local logs.":
            "Вход истёк — откройте Codex, чтобы войти снова. Лимиты по локальным журналам.",
        "Credit balance needs a management key": "Для баланса кредитов нужен ключ управления",
        "Credit balance is unavailable right now": "Баланс кредитов сейчас недоступен",
        "No Cline credits left.": "Кредиты Cline закончились.",
        "Legacy Grok billing has no weekly pool.": "В старом тарифе Grok нет недельного лимита.",
        "Ollama reports no Cloud limits for this account yet.":
            "Ollama пока не сообщает лимиты Cloud для этого аккаунта.",
        "Could not read the Ollama plan; the usage above is up to date.":
            "Не удалось прочитать тариф Ollama; использование выше актуально.",
        "Antigravity reports no quota pools for this account.": "Antigravity не сообщает лимиты для этого аккаунта.",
        "The keyring that holds the Antigravity sign-in is locked. Unlock it or start Antigravity.":
            "Связка ключей со входом в Antigravity заблокирована. Разблокируйте её или запустите Antigravity.",
    ]

    private static let extraUsagePrefix = "Extra usage on, cap "
    private static let utcSuffix = " (UTC)."

    public static func translate(_ text: String, language: UILanguage) -> String {
        guard language == .ru else { return text }
        if let fixed = russianFixed[text] { return fixed }
        if text.hasPrefix(extraUsagePrefix) {
            return "Доп. использование включено, предел \(text.dropFirst(extraUsagePrefix.count))"
        }
        return planDate(text) ?? text
    }

    private static func planDate(_ text: String) -> String? {
        guard text.hasSuffix(utcSuffix) else { return nil }
        let body = String(text.dropLast(utcSuffix.count))
        for (english, russian) in [(" renews on ", "продлевается"), (" ends on ", "заканчивается")] {
            guard let range = body.range(of: english, options: .backwards) else { continue }
            let plan = body[..<range.lowerBound]
            let date = body[range.upperBound...]
            guard !plan.isEmpty, !date.isEmpty, !date.contains(" ") else { continue }
            return "\(plan) \(russian) \(date) (UTC)."
        }
        return nil
    }
}
