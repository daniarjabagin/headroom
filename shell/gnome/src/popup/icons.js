import { fileIcon, themeIcon } from '../widgets.js';

const FILE_ICONS = {
    status: 'status-page-symbolic.svg',
    chart: 'chart-symbolic.svg',
    external: 'link-external-symbolic.svg',
    share: 'share-symbolic.svg',
    'check-circle': 'check-circle-symbolic.svg',
};

export function popupIcon(dir, key, styleClass) {
    const file = FILE_ICONS[key];
    return file ? fileIcon(dir, file, styleClass) : themeIcon(`${key}-symbolic`, styleClass);
}
