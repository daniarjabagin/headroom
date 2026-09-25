pub(super) const CATALOG: &[(&str, &str)] = &[
    ("Time format", "Формат времени"),
    (
        "Exact reset times and chart labels",
        "Точное время сброса и подписи графиков",
    ),
    ("Density", "Плотность"),
    (
        "Compact fits more accounts in the popup",
        "В компактном виде в окно помещается больше аккаунтов",
    ),
    ("Tray icon shows", "Значок в трее показывает"),
    (
        "What the icon in the tray draws",
        "Что рисует значок в трее",
    ),
    ("Limits in the icon", "Лимиты в значке"),
    (
        "None chosen · the two most critical are shown",
        "Ничего не выбрано · показаны два самых критичных",
    ),
    (
        "{count} of {max} chosen · shown in this order",
        "Выбрано {count} из {max} · в этом порядке",
    ),
    ("Indicator style", "Стиль индикатора"),
    (
        "The mark in front of each figure",
        "Значок перед каждым числом",
    ),
    ("The text next to each mark", "Текст рядом с каждым значком"),
    ("Spend", "Траты"),
    ("Show spend", "Показывать траты"),
    (
        "Spend ring for all tools at the top of the popup",
        "Кольцо трат по всем инструментам вверху окна",
    ),
    ("Default period", "Период по умолчанию"),
    (
        "The tab the spend ring opens on",
        "Вкладка, с которой открывается кольцо трат",
    ),
    ("Units", "Единицы"),
    ("Breakdown on hover", "Разбивка при наведении"),
    (
        "Split a slice of the ring when the pointer rests on it",
        "Показывает состав доли кольца под указателем",
    ),
    ("Popup Cards", "Карточки в окне"),
    (
        "Star the accounts that always show their limits.",
        "Отметьте звёздочкой аккаунты, лимиты которых видны всегда.",
    ),
    (
        "Collapse unstarred accounts",
        "Сворачивать аккаунты без звёздочки",
    ),
    (
        "They fold into one line in the popup and open on click",
        "Они собираются в одну строку и раскрываются по нажатию",
    ),
    ("Always open", "Всегда открыт"),
    ("On demand", "По запросу"),
    ("Always open in the popup", "Всегда открыт в окне"),
    (
        "Faster while coding tools run",
        "Чаще, пока работают инструменты",
    ),
    (
        "Every minute while Claude Code, Codex or another tracked tool is writing",
        "Раз в минуту, пока Claude Code, Codex или другой инструмент пишет логи",
    ),
    ("Privacy", "Конфиденциальность"),
    ("Latest version", "Последняя версия"),
    ("Status pages", "Страницы статуса"),
    (
        "Show provider incidents from public status pages",
        "Показывать сбои провайдеров с публичных страниц статуса",
    ),
    ("Keyboard", "Клавиатура"),
    (
        "Opens the popup from any app",
        "Открывает окно из любого приложения",
    ),
    ("Disabled", "Выключено"),
    ("Set Shortcut", "Сочетание клавиш"),
    ("Press your keyboard shortcut…", "Нажмите сочетание клавиш…"),
    (
        "Press Esc to cancel or Backspace to disable the keyboard shortcut.",
        "Esc — отмена, Backspace — выключить сочетание.",
    ),
    ("Current", "Сейчас"),
    (
        "A limit drops under {percent}% left",
        "Остаток лимита опускается ниже {percent}%",
    ),
    ("Alert Threshold", "Порог уведомлений"),
    ("Alert when less than", "Уведомлять, когда осталось меньше"),
    ("Used by Almost out", "Для уведомления «Почти исчерпан»"),
    ("Per provider", "Для каждого провайдера"),
    (
        "Every provider uses the default",
        "У всех провайдеров общий порог",
    ),
    ("Quiet Hours", "Тихие часы"),
    (
        "Held notifications arrive together when quiet hours end.",
        "Отложенные уведомления придут вместе, когда тихие часы закончатся.",
    ),
    ("Quiet hours", "Тихие часы"),
    (
        "Hold notifications while you are away",
        "Откладывать уведомления, пока вас нет",
    ),
    ("From", "С"),
    ("To", "До"),
    (
        "Still show critical alerts",
        "Всё равно показывать критичные",
    ),
    (
        "Will run out and Almost out come through",
        "«Закончится раньше сброса» и «Почти исчерпан» приходят сразу",
    ),
    ("Advanced", "Дополнительно"),
    ("Restart", "Перезапустить"),
    ("version", "версия"),
    ("systemd user service", "пользовательская служба systemd"),
    ("Logging", "Журнал"),
    ("Log level", "Уровень журнала"),
    (
        "Debug adds provider responses without tokens",
        "Отладка добавляет ответы провайдеров без токенов",
    ),
    ("Log file", "Файл журнала"),
    ("Not available", "Недоступно"),
    ("Copy path", "Скопировать путь"),
    ("Path copied", "Путь скопирован"),
    ("Open folder", "Открыть папку"),
    ("Troubleshooting", "Диагностика"),
    (
        "Paste the diagnostics into a bug report.",
        "Вставьте диагностику в сообщение об ошибке.",
    ),
    ("Copy diagnostics", "Скопировать диагностику"),
    (
        "Versions, desktop and account states. No tokens or emails.",
        "Версии, окружение и состояние аккаунтов. Без токенов и почты.",
    ),
    ("Diagnostics copied", "Диагностика скопирована"),
    ("Reset all settings…", "Сбросить все настройки…"),
    ("Reset all settings?", "Сбросить все настройки?"),
    (
        "Accounts stay signed in; appearance, notifications and hidden limits return to defaults.",
        "Аккаунты останутся подключены; вид, уведомления и скрытые лимиты вернутся к исходным.",
    ),
    ("Reset", "Сбросить"),
    ("Settings reset", "Настройки сброшены"),
    (
        "Restarting the Headroom service…",
        "Служба Headroom перезапускается…",
    ),
    (
        "The Headroom service could not be restarted",
        "Не удалось перезапустить службу Headroom",
    ),
    ("Sign In to {provider} Again", "Повторный вход в {provider}"),
    (
        "Signed in again. The account updates in a moment.",
        "Вход выполнен. Аккаунт скоро обновится.",
    ),
];

pub(super) const PLURALS: &[(&str, [&str; 3])] = &[(
    "{count} provider differs from the default",
    [
        "{count} провайдер отличается от общего",
        "{count} провайдера отличаются от общего",
        "{count} провайдеров отличаются от общего",
    ],
)];
