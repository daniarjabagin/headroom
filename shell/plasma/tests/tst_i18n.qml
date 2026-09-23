import QtQuick
import QtTest
import "../package/contents/ui/logic/I18n.js" as I18n
import "../package/contents/ui/logic/Russian.js" as Russian

TestCase {
    function test_resolve() {
        compare(I18n.resolve("system", "ru_RU"), "ru");
        compare(I18n.resolve("system", "de_DE"), "en");
        compare(I18n.resolve("en", "ru_RU"), "en");
        compare(I18n.resolve("ru", "en_US"), "ru");
        compare(I18n.resolve("klingon", "C"), "en");
    }

    function test_translate() {
        compare(I18n.tr("en", "{percent}% left", {
            percent: 62
        }), "62% left");
        compare(I18n.tr("ru", "{percent}% left", {
            percent: 62
        }), "Осталось 62%");
        compare(I18n.tr("ru", "Settings"), "Настройки");
        compare(I18n.tr("ru", "Refresh"), "Обновить");
        compare(I18n.tr("ru", "Accounts"), "Аккаунты");
        compare(I18n.tr("ru", "Notifications"), "Уведомления");
        compare(I18n.tr("ru", "not in the catalog {x}", {
            x: 1
        }), "not in the catalog 1");
        compare(I18n.fill("{a} {b}", {
            a: 1
        }), "1 {b}");
    }

    function test_russian_plural_rules_data() {
        return [0, 1, 2, 4, 5, 11, 12, 14, 21, 22, 25, 101, 111, 112, 1001].map(count => ({
                    tag: String(count),
                    count,
                    index: [1, 21, 101, 1001].includes(count) ? 0 : ([2, 4, 22].includes(count) ? 1 : 2)
                }));
    }

    function test_russian_plural_rules(row) {
        compare(I18n.pluralIndex("ru", row.count), row.index);
    }

    function test_plurals() {
        compare(I18n.pluralIndex("en", 1), 0);
        compare(I18n.pluralIndex("en", 2), 1);
        compare(I18n.trn("ru", "{count} second", "{count} seconds", 21), "21 секунда");
        compare(I18n.trn("ru", "{count} second", "{count} seconds", 3), "3 секунды");
        compare(I18n.trn("ru", "{count} second", "{count} seconds", 11), "11 секунд");
        compare(I18n.trn("en", "{count} second", "{count} seconds", 1), "1 second");
        compare(I18n.trn("en", "{count} second", "{count} seconds", 5), "5 seconds");
    }

    function test_catalog_shape() {
        for (const [msgid, entry] of Object.entries(Russian.MESSAGES)) {
            if (Array.isArray(entry))
                compare(entry.length, 3, msgid);
            else
                compare(typeof entry, "string", msgid);
        }
    }
}
