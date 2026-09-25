const CATALOG: &[(&str, &str)] = &[
    ("Session", "Сессия"),
    ("Weekly", "Неделя"),
    ("{percent}% left", "Осталось {percent}%"),
    ("{percent}% used", "Использовано {percent}%"),
    ("{seconds}s", "{seconds} с"),
    ("{minutes}m {seconds}s", "{minutes} мин {seconds} с"),
    ("{days}d {hours}h", "{days} д {hours} ч"),
    ("{hours}h {minutes}m", "{hours} ч {minutes} мин"),
    ("{minutes}m", "{minutes} мин"),
    ("resets {moment}", "сброс {moment}"),
    ("resets soon", "скоро сброс"),
    ("reset pending", "ожидается сброс"),
    ("resets in {duration}", "сброс через {duration}"),
    ("Not started", "Не начато"),
    ("~{percent}% spare", "~{percent}% запаса"),
    ("Limit soon", "Скоро лимит"),
    ("Limit in {duration}", "Лимит через {duration}"),
    (
        "At this pace: runs out any minute",
        "При текущем темпе закончится с минуты на минуту",
    ),
    ("runs out in {duration}", "закончится через {duration}"),
    ("At this pace: {runsOut}", "При текущем темпе: {runsOut}"),
    (
        "At this pace: {runsOut} · {resets}",
        "При текущем темпе: {runsOut} · {resets}",
    ),
    (
        "At this pace: ~{percent}% used at reset",
        "При текущем темпе к сбросу будет использовано ~{percent}%",
    ),
    (
        "At this pace: ~{percent}% left at reset",
        "При текущем темпе к сбросу останется ~{percent}%",
    ),
    ("Next update in <1m", "Обновление через <1 мин"),
    ("Next update in {duration}", "Обновление через {duration}"),
    ("just now", "только что"),
    ("{duration} ago", "{duration} назад"),
    ("{month} {date}", "{date} {month}"),
    ("today at {time}", "сегодня в {time}"),
    ("tomorrow at {time}", "завтра в {time}"),
    ("{weekday} at {time}", "{weekday} в {time}"),
    ("{day} at {time}", "{day} в {time}"),
    ("unpriced", "без цены"),
    ("Other ({count})", "Другие ({count})"),
    ("Service not running", "Служба не запущена"),
    ("Connecting…", "Подключение…"),
    (
        "Offline — last update {time}",
        "Нет сети — обновлено в {time}",
    ),
    ("Offline", "Нет сети"),
    ("Updating…", "Обновление…"),
    ("Updated {time}", "Обновлено в {time}"),
    ("Refresh", "Обновить"),
    ("Limit reached", "Лимит исчерпан"),
    ("Over pace", "Темп превышен"),
    ("No data", "Нет данных"),
    ("Credits", "Кредиты"),
    ("Extra usage", "Доп. использование"),
    ("Balance", "Баланс"),
    ("Vouchers", "Ваучеры"),
    ("Cash", "Денежный баланс"),
    ("Credit balance", "Кредиты"),
    ("Organization credits", "Кредиты организации"),
    ("Point balance", "Баланс баллов"),
    ("Bonus credits", "Бонусные кредиты"),
    ("Monthly credits", "Кредиты на месяц"),
    (" · some models unpriced", " · часть моделей без цены"),
    ("No usage", "Без использования"),
    ("Usage Trend", "Динамика"),
    ("Today", "Сегодня"),
    ("Yesterday", "Вчера"),
    ("Last 30 Days", "За 30 дней"),
    ("30 Days", "30 дней"),
    ("Outdated", "Устарело"),
    ("Last updated {ago}", "Обновлено {ago}"),
    ("Refresh failed", "Не удалось обновить"),
    ("Retry", "Повторить"),
    ("Retrying…", "Повторяем…"),
    ("Sign in…", "Войти…"),
    ("Sign in again…", "Войти снова…"),
    ("Copy command", "Скопировать команду"),
    (
        "Run `{command}` in a terminal — Headroom picks it up automatically.",
        "Выполните `{command}` в терминале — Headroom подхватит вход сам.",
    ),
    (
        "Another account is signed in to {provider}",
        "В {provider} выполнен вход в другой аккаунт",
    ),
    ("Signed out of {provider}", "Выполнен выход из {provider}"),
    (
        "Sign in again to keep this account up to date.",
        "Войдите снова, чтобы данные этого аккаунта обновлялись.",
    ),
    (
        "Couldn't refresh {provider}",
        "Не удалось обновить {provider}",
    ),
    ("No active subscription", "Подписка неактивна"),
    (
        "Limits aren't available for this account. Renew the plan or sign in with another account.",
        "Данные о лимитах недоступны. Продлите подписку или войдите в другой аккаунт.",
    ),
    ("No usage in this period", "За этот период расходов нет"),
    ("Total Spend", "Всего потрачено"),
    (
        "Some models have no public price yet.",
        "У некоторых моделей пока нет публичной цены.",
    ),
    (
        "Estimated from local logs and public pricing.",
        "Оценка по локальным журналам и публичным ценам.",
    ),
    (
        "Partly unpriced, cost leaves it out",
        "Частично без цены, в стоимость не входит",
    ),
    (
        "Headroom service isn't running",
        "Служба Headroom не запущена",
    ),
    (
        "Start it to see your usage limits here.",
        "Запустите её, чтобы видеть здесь свои лимиты.",
    ),
    ("Starting…", "Запуск…"),
    ("Start service", "Запустить службу"),
    ("Try again", "Повторить"),
    (
        "Couldn't read Headroom's state",
        "Не удалось прочитать состояние Headroom",
    ),
    ("Check again", "Проверить снова"),
    (
        "No AI coding tools found.",
        "ИИ-инструменты для программирования не найдены.",
    ),
    (
        "Weekly limit shared with Codex Cloud",
        "Недельный лимит общий с Codex Cloud",
    ),
    (
        "Offline — showing limits from local logs.",
        "Нет сети — лимиты по локальным журналам.",
    ),
    (
        "Sign-in expired — open Codex to sign in again. Showing limits from local logs.",
        "Вход истёк — откройте Codex, чтобы войти снова. Лимиты по локальным журналам.",
    ),
    (
        "Credit balance needs a management key",
        "Для баланса кредитов нужен ключ управления",
    ),
    (
        "Credit balance is unavailable right now",
        "Баланс кредитов сейчас недоступен",
    ),
    ("No Cline credits left.", "Кредиты Cline закончились."),
    (
        "Legacy Grok billing has no weekly pool.",
        "В старом тарифе Grok нет недельного лимита.",
    ),
    (
        "Ollama reports no Cloud limits for this account yet.",
        "Ollama пока не сообщает лимиты Cloud для этого аккаунта.",
    ),
    (
        "Could not read the Ollama plan; the usage above is up to date.",
        "Не удалось прочитать тариф Ollama; использование выше актуально.",
    ),
    (
        "Antigravity reports no quota pools for this account.",
        "Antigravity не сообщает лимиты для этого аккаунта.",
    ),
    (
        "The keyring that holds the Antigravity sign-in is locked. Unlock it or start Antigravity.",
        "Связка ключей со входом в Antigravity заблокирована. Разблокируйте её или запустите Antigravity.",
    ),
    ("Kilo credits are used up", "Кредиты Kilo закончились"),
    ("Unlimited credits", "Безлимитные кредиты"),
    (
        "No monthly credits on this plan",
        "В этом тарифе нет ежемесячных кредитов",
    ),
    (
        "Balance is not enough for API calls",
        "Баланса не хватает для вызовов API",
    ),
    (
        "Balance is used up; API calls fail until you top up",
        "Баланс исчерпан — вызовы API не пройдут, пока вы не пополните счёт",
    ),
    (
        "Balance is used up; API requests fail until you top up",
        "Баланс исчерпан — запросы к API не пройдут, пока вы не пополните счёт",
    ),
    (
        "Cash balance is negative: the account is in debt",
        "Денежный баланс отрицательный — на счёте долг",
    ),
    (
        "Extra usage on, cap {amount}",
        "Доп. использование включено, предел {amount}",
    ),
    (
        "{plan} renews on {date} (UTC).",
        "{plan} продлевается {date} (UTC).",
    ),
    (
        "{plan} ends on {date} (UTC).",
        "{plan} заканчивается {date} (UTC).",
    ),
    ("Update", "Обновить"),
    ("How to update", "Как обновить"),
    ("Release notes", "Примечания к выпуску"),
    ("What's new", "Что нового"),
    (
        "Headroom {version} is available",
        "Доступна версия Headroom {version}",
    ),
    ("Starting the update…", "Запускаем обновление…"),
    (
        "Updated — log out and back in to finish",
        "Обновлено — выйдите из сеанса и войдите снова",
    ),
    ("Updated to {version}", "Обновлено до {version}"),
    ("Updated", "Обновлено"),
    (
        "The update stopped before it finished",
        "Обновление прервалось и не завершилось",
    ),
    ("Copy", "Копировать"),
    ("Copied", "Скопировано"),
    ("Open Headroom", "Открыть Headroom"),
    ("Refresh now", "Обновить сейчас"),
    ("Quit", "Выйти"),
    ("Loading…", "Загрузка…"),
    ("No usage limits to show", "Нет лимитов для показа"),
    ("Press Enter to close", "Нажмите Enter, чтобы закрыть"),
    (
        "{percent}% left of {capacity}%",
        "Осталось {percent}% из {capacity}%",
    ),
    (
        "{percent}% used of {capacity}%",
        "Использовано {percent}% из {capacity}%",
    ),
];

const PLURALS: &[(&str, [&str; 3])] = &[
    (
        "{tokens} token",
        ["{tokens} токен", "{tokens} токена", "{tokens} токенов"],
    ),
    (
        "{count} account",
        ["{count} аккаунт", "{count} аккаунта", "{count} аккаунтов"],
    ),
];

pub(super) fn lookup(msgid: &str) -> Option<&'static str> {
    CATALOG
        .iter()
        .chain(super::ru_prefs::CATALOG)
        .chain(super::ru_options::CATALOG)
        .chain(super::ru_popup::CATALOG)
        .find(|(key, _)| *key == msgid)
        .map(|(_, value)| *value)
}

pub(super) fn plural(msgid: &str) -> Option<[&'static str; 3]> {
    PLURALS
        .iter()
        .chain(super::ru_prefs::PLURALS)
        .chain(super::ru_options::PLURALS)
        .chain(super::ru_popup::PLURALS)
        .find(|(key, _)| *key == msgid)
        .map(|(_, forms)| *forms)
}
