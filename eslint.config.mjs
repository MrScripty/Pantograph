import js from '@eslint/js';
import svelte from 'eslint-plugin-svelte';
import tseslint from 'typescript-eslint';
import globals from 'globals';
import noSvgTextContent from './eslint-rules/no-svg-text-content.mjs';

// Custom plugin for Pantograph-specific rules
const pantographPlugin = {
  rules: {
    'no-svg-text-content': noSvgTextContent,
  },
};

export default [
  {
    // Flat-config ignores must be in a standalone object.
    ignores: [
      'node_modules/**',
      'dist/**',
      'target/**',
      '.venv/**',
      'src-tauri/**',
    ],
  },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  ...svelte.configs['flat/recommended'],
  {
    files: ['scripts/**/*.mjs', '*.config.{js,mjs,cjs}', 'vite.config.ts', 'eslint.config.mjs'],
    languageOptions: {
      globals: {
        ...globals.node,
      },
    },
    rules: {
      'no-unused-vars': [
        'error',
        {
          argsIgnorePattern: '^_',
          varsIgnorePattern: '^_',
          caughtErrors: 'none',
        },
      ],
      '@typescript-eslint/no-unused-vars': 'off',
    },
  },
  {
    files: ['src/**/*.{ts,svelte}', 'packages/svelte-graph/src/**/*.{ts,svelte}'],
    languageOptions: {
      globals: {
        ...globals.browser,
      },
    },
    rules: {
      // Use TS-aware variant; base rule causes duplicate reports on TS files.
      'no-unused-vars': 'off',
      '@typescript-eslint/no-unused-vars': [
        'error',
        { argsIgnorePattern: '^_', varsIgnorePattern: '^_' },
      ],
    },
  },
  {
    files: ['src/generated/**/*.svelte'],
    plugins: {
      pantograph: pantographPlugin,
    },
    rules: {
      // Keep stricter guardrails for generated components.
      'no-undefined': 'error',
      'no-implicit-coercion': 'error',
      'no-unused-expressions': 'error',
      'pantograph/no-svg-text-content': 'error',
    },
  },
  {
    files: ['**/*.svelte'],
    languageOptions: {
      parserOptions: {
        parser: tseslint.parser,
      },
    },
    rules: {
      // Svelte 5 uses runes like $state, $derived which look like undefined globals
      // These are compile-time constructs, not runtime variables
      'no-undef': 'off',
    },
  }
];
