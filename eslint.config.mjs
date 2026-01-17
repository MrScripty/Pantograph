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
  js.configs.recommended,
  ...tseslint.configs.recommended,
  ...svelte.configs['flat/recommended'],
  {
    languageOptions: {
      globals: {
        ...globals.browser,
      },
    },
    plugins: {
      pantograph: pantographPlugin,
    },
    rules: {
      // Catch suspicious undefined usage - explicit undefined assignment is almost always a mistake
      'no-undefined': 'error',
      // Catch unused variables (ignore underscore-prefixed for intentional unused params)
      'no-unused-vars': ['error', { argsIgnorePattern: '^_' }],
      // Catch implicit type coercion (e.g., +value, !!value)
      'no-implicit-coercion': 'error',
      // Catch unused expressions that don't affect anything
      'no-unused-expressions': 'error',
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
      // Catch string interpolation inside SVG elements (common agent mistake)
      'pantograph/no-svg-text-content': 'error',
    },
  },
  {
    // Ignore generated files and node_modules
    ignores: ['node_modules/**', 'dist/**', 'src-tauri/**'],
  },
];
