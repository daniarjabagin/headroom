public enum WelcomeText: LocalizedText {
    case windowTitle, title, menuBarHint, openAtLogin, openAtLoginDetail, accountsNote, openHeadroom

    public var translations: (english: String, russian: String) {
        switch self {
        case .windowTitle: ("Welcome to Headroom", "Добро пожаловать в Headroom")
        case .title: ("Headroom is in your menu bar", "Headroom — в строке меню")
        case .menuBarHint:
            (
                "Look for its icon at the top right of the screen. Click it to see how much of your AI coding limits is left and when they reset.",
                "Его значок — в правом верхнем углу экрана. Нажмите на него, чтобы увидеть, сколько осталось от лимитов ИИ-ассистентов и когда они сбросятся."
            )
        case .openAtLogin: ("Open at login", "Открывать при входе")
        case .openAtLoginDetail:
            (
                "Keep Headroom in the menu bar after you restart this Mac",
                "Headroom останется в строке меню после перезагрузки Mac"
            )
        case .accountsNote:
            (
                "Accounts from the Claude Code and Codex CLIs are detected automatically. Add others in Settings → Accounts.",
                "Аккаунты из Claude Code и Codex CLI находятся автоматически. Остальные можно добавить в Настройках → Аккаунты."
            )
        case .openHeadroom: ("Open Headroom", "Открыть Headroom")
        }
    }
}
