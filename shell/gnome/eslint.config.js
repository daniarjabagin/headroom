import js from '@eslint/js';
import prettier from 'eslint-config-prettier';

const gjsGlobals = {
    global: 'readonly',
    console: 'readonly',
    log: 'readonly',
    logError: 'readonly',
    print: 'readonly',
    printerr: 'readonly',
    imports: 'readonly',
    ARGV: 'readonly',
    TextDecoder: 'readonly',
    TextEncoder: 'readonly',
};

export default [
    { ignores: ['node_modules/', 'build/'] },
    js.configs.recommended,
    {
        files: ['**/*.js'],
        languageOptions: {
            ecmaVersion: 2024,
            sourceType: 'module',
            globals: gjsGlobals,
        },
        rules: {
            'no-unused-vars': ['error', { argsIgnorePattern: '^_', varsIgnorePattern: '^_' }],
            'no-var': 'error',
            'prefer-const': 'error',
            eqeqeq: ['error', 'always'],
            'no-warning-comments': 'error',
            'max-lines': ['error', { max: 400, skipBlankLines: true }],
            'max-lines-per-function': ['warn', { max: 40, skipBlankLines: true }],
        },
    },
    prettier,
];
