import Gio from 'gi://Gio';
import GLib from 'gi://GLib';

export function loadBytes(file) {
    return new Promise((resolve, reject) => {
        file.load_contents_async(null, (source, result) => {
            try {
                resolve(source.load_contents_finish(result)[1]);
            } catch (error) {
                reject(error);
            }
        });
    });
}

export function picturesFolder() {
    const pictures =
        GLib.get_user_special_dir(GLib.UserDirectory.DIRECTORY_PICTURES) ??
        GLib.build_filenamev([GLib.get_home_dir(), 'Pictures']);
    return Gio.File.new_for_path(GLib.build_filenamev([pictures, 'Headroom']));
}

function twoDigits(value) {
    return String(value).padStart(2, '0');
}

export function shareFileName(provider, now) {
    const date = `${now.getFullYear()}${twoDigits(now.getMonth() + 1)}${twoDigits(now.getDate())}`;
    const time = `${twoDigits(now.getHours())}${twoDigits(now.getMinutes())}${twoDigits(now.getSeconds())}`;
    return `headroom-${provider}-${date}-${time}.png`;
}
