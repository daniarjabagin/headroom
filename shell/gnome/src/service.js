import Gio from 'gi://Gio';

const START_COMMAND = ['systemctl', '--user', 'start', 'headroom.service'];

function communicate(process, cancellable) {
    return new Promise((resolve, reject) => {
        process.communicate_utf8_async(null, cancellable, (source, result) => {
            try {
                resolve(source.communicate_utf8_finish(result));
            } catch (error) {
                reject(error);
            }
        });
    });
}

export async function startService(cancellable) {
    const process = Gio.Subprocess.new(
        START_COMMAND,
        Gio.SubprocessFlags.STDOUT_SILENCE | Gio.SubprocessFlags.STDERR_PIPE
    );
    const [, , stderr] = await communicate(process, cancellable);
    if (!process.get_successful()) throw new Error(stderr?.trim() || 'systemctl could not start headroom.service');
}
