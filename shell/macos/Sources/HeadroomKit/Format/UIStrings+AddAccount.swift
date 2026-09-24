public enum AddAccountText: LocalizedText {
    case addAccount, chooseService, loadingProviders, noProviders, noProvidersDetail, methodQuestion
    case signInWith, signIn, apiKey, detectedAutomatically, addProviderAccount, labelOptional, close
    case autoDetectFallback, detectAgainNote, detectAgain, lookingForAccounts, scanFinished, serviceUnreachable
    case missingHelper, exitStatus

    public var translations: (english: String, russian: String) {
        switch self {
        case .addAccount: ("Add Account", "Добавить аккаунт")
        case .chooseService: ("Choose the service to track.", "Выберите сервис для отслеживания.")
        case .loadingProviders: ("Loading services…", "Загрузка сервисов…")
        case .noProviders: ("No providers available", "Нет доступных сервисов")
        case .noProvidersDetail:
            ("The Headroom service did not list any providers.", "Служба Headroom не сообщила ни одного сервиса.")
        case .methodQuestion: ("How do you want to add your {provider} account?", "Как добавить аккаунт {provider}?")
        case .signInWith: ("Sign in with {program}", "Вход через {program}")
        case .signIn: ("Sign in", "Вход")
        case .apiKey: ("API key", "API-ключ")
        case .detectedAutomatically: ("Detected automatically", "Находится автоматически")
        case .addProviderAccount: ("Add {provider} Account", "Новый аккаунт {provider}")
        case .labelOptional: ("Label (optional)", "Название (необязательно)")
        case .close: ("Close", "Закрыть")
        case .autoDetectFallback: ("Headroom finds this account on its own.", "Headroom находит этот аккаунт сам.")
        case .detectAgainNote:
            (
                "Already set up, or removed it earlier? Detect again to find it.",
                "Уже настроили или удалили раньше? Запустите поиск, чтобы найти его снова."
            )
        case .detectAgain: ("Detect Again", "Искать снова")
        case .lookingForAccounts: ("Looking for accounts…", "Ищем аккаунты…")
        case .scanFinished:
            (
                "Scan finished. Accounts that were found appear in the list.",
                "Поиск завершён. Найденные аккаунты появятся в списке."
            )
        case .serviceUnreachable: ("Couldn't reach the Headroom service.", "Не удалось связаться со службой Headroom.")
        case .missingHelper:
            (
                "The headroom helper is missing from the app. Reinstall Headroom.",
                "В приложении нет помощника headroom. Переустановите Headroom."
            )
        case .exitStatus: ("headroom exited with status {status}", "headroom завершился с кодом {status}")
        }
    }
}

public enum SignInText: LocalizedText {
    case signInTo, cliIntro, continueAction, startingSignIn, takesAMoment, waitingForSignIn, openPageHint
    case openSignInPage, pasteCode, pasteCodeHint, send, codeSent, yourCode, copy, cancel, done, accountAdded
    case tryAgain, connectProvider, apiKeyIntro, getKey, add, checkingKey

    public var translations: (english: String, russian: String) {
        switch self {
        case .signInTo: ("Sign in to {provider}", "Вход в {provider}")
        case .cliIntro:
            (
                "Headroom signs in with the {provider} CLI in its own folder, so the account you use today stays signed in.",
                "Headroom входит через CLI {provider} в отдельной папке, поэтому ваш текущий аккаунт останется активным."
            )
        case .continueAction: ("Continue", "Продолжить")
        case .startingSignIn: ("Starting sign-in…", "Начинаем вход…")
        case .takesAMoment: ("This takes a moment.", "Это займёт немного времени.")
        case .waitingForSignIn: ("Waiting for sign-in…", "Ожидание входа…")
        case .openPageHint:
            (
                "Open the sign-in page and finish in your browser.",
                "Откройте страницу входа и завершите вход в браузере."
            )
        case .openSignInPage: ("Open Sign-In Page", "Открыть страницу входа")
        case .pasteCode: ("Paste code", "Вставьте код")
        case .pasteCodeHint: ("Only if the sign-in page shows a code.", "Только если страница входа показывает код.")
        case .send: ("Send", "Отправить")
        case .codeSent: ("Code sent", "Код отправлен")
        case .yourCode: ("Your code", "Ваш код")
        case .copy: ("Copy", "Скопировать")
        case .cancel: ("Cancel", "Отмена")
        case .done: ("Done", "Готово")
        case .accountAdded:
            (
                "Account added. It shows up in the menu bar in a moment.",
                "Аккаунт добавлен. Скоро он появится в строке меню."
            )
        case .tryAgain: ("Try Again", "Попробовать снова")
        case .connectProvider: ("Connect {provider}", "Подключение {provider}")
        case .apiKeyIntro:
            (
                "Headroom checks the key with {provider} and keeps it in your Keychain. It never leaves this Mac.",
                "Headroom проверит ключ в {provider} и сохранит его в Связке ключей. Ключ не покидает этот Mac."
            )
        case .getKey: ("Get a Key", "Получить ключ")
        case .add: ("Add", "Добавить")
        case .checkingKey: ("Checking the key…", "Проверяем ключ…")
        }
    }
}
