module.exports = {
  content: ['./index.html', './src/**/*.{svelte,ts}'],
  theme: {
    extend: {
      colors: {
        neutral: {
          50: '#fafafa',
          100: '#f5f5f5',
          200: '#e5e5e5',
          300: '#d4d4d4',
          400: '#b8b8b8',
          500: '#969696',
          600: '#777777',
          700: '#5d5d5d',
          800: '#464646',
          900: '#383838',
          950: '#2f2f2f',
        },
      },
    },
  },
  plugins: [],
};
