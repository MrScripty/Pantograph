// Workbench tokens live as CSS custom properties in src/styles.css; these
// colours point at them so `bg-pg-panel`, `text-pg-text-muted`,
// `border-pg-border` and friends follow the one palette. Opacity modifiers
// (`bg-pg-panel/50`) are not supported for the `pg` colours because the
// variables hold hex values.
const pg = (name) => `var(--pg-${name})`;

module.exports = {
  content: ['./index.html', './src/**/*.{svelte,ts}'],
  theme: {
    extend: {
      colors: {
        pg: {
          ground: pg('ground'),
          panel: pg('panel'),
          'panel-raised': pg('panel-raised'),
          'panel-selected': pg('panel-selected'),
          'panel-sunken': pg('panel-sunken'),
          'panel-head': pg('panel-head'),
          border: pg('border'),
          'border-soft': pg('border-soft'),
          'border-strong': pg('border-strong'),
          text: pg('text'),
          'text-bright': pg('text-bright'),
          'text-muted': pg('text-muted'),
          'text-soft': pg('text-soft'),
          accent: pg('accent'),
          danger: pg('danger'),
          ok: pg('ok'),
          info: pg('info'),
          ink: pg('ink'),
        },
        // Pages that are not yet restyled still use the neutral scale. It is
        // tuned to the workbench greys (950 = ground, 900 = panel) so those
        // pages sit on the new palette without `!important` overrides.
        neutral: {
          50: '#f4f5f7',
          100: '#e4e6ea',
          200: '#d5d9df',
          300: '#c9cdd4',
          400: '#a3a9b4',
          500: '#8a909a',
          600: '#6b717b',
          700: '#4a5059',
          800: '#3a3f47',
          900: '#33383f',
          950: '#2a2e35',
        },
      },
      fontFamily: {
        serif: ['Newsreader', 'Georgia', 'serif'],
        sans: ['"Atkinson Hyperlegible"', 'system-ui', 'sans-serif'],
        mono: ['"JetBrains Mono"', 'ui-monospace', 'monospace'],
      },
    },
  },
  plugins: [],
};
