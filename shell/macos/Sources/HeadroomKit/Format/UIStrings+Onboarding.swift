public enum OnboardingText: LocalizedText {
    case welcome, tagline, whatWeFound, whatWeFoundDetail, signedIn, foundNotSignedIn, notInstalled, signIn,
        addAnotherAccount, start, chooseLater, nothingFound

    public var translations: (english: String, russian: String) {
        switch self {
        case .welcome: ("Welcome to Headroom", "Добро пожаловать в Headroom")
        case .tagline: ("Your AI coding limits, right next to the clock.", "Лимиты ИИ-ассистентов — рядом с часами.")
        case .whatWeFound: ("Here's what we found", "Вот что мы нашли")
        case .whatWeFoundDetail:
            ("Turn off anything you don't want to track.", "Выключите то, что не нужно отслеживать.")
        case .signedIn: ("Signed in", "Вход выполнен")
        case .foundNotSignedIn: ("Found, not signed in", "Найден, вход не выполнен")
        case .notInstalled: ("Not installed", "Не установлен")
        case .signIn: ("Sign In", "Войти")
        case .addAnotherAccount: ("Add another account…", "Добавить другой аккаунт…")
        case .start: ("Start", "Начать")
        case .chooseLater: ("Choose later", "Выбрать позже")
        case .nothingFound:
            ("No coding tools found yet. Add an account to start.", "ИИ-инструменты пока не найдены. Добавьте аккаунт.")
        }
    }
}

public enum SignInAgainText: LocalizedText {
    case signInAgain, signInAgainDetail, signedInAgain, signInAgainTitle

    public var translations: (english: String, russian: String) {
        switch self {
        case .signInAgain: ("Sign In Again…", "Войти снова…")
        case .signInAgainDetail: ("The sign-in for this account has expired", "Срок входа в этот аккаунт истёк")
        case .signedInAgain: ("Signed in again", "Вход выполнен снова")
        case .signInAgainTitle: ("Sign in to {provider} again", "Снова войти в {provider}")
        }
    }
}
