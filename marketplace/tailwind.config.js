/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
  theme: {
    extend: {
      colors: Object.fromEntries(['page', 'surface', 'muted', 'ink', 'secondary', 'line', 'accent', 'on-accent'].map(name => [name, `rgb(var(--${name}) / <alpha-value>)`])),
      fontFamily: {
        sans: ['"IBM Plex Sans"', 'system-ui', '-apple-system', 'sans-serif'],
        mono: ['"IBM Plex Mono"', 'Menlo', 'monospace'],
      },
      fontSize: { xs: ['0.875rem', '1.5'], sm: ['0.9375rem', '1.6'] },
    },
  },
  plugins: [],
};
