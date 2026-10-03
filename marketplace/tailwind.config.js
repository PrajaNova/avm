/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        // Overrides Tailwind's built-in `slate`/`emerald` scales rather than
        // adding new token names — the app uses slate-*/emerald-* directly
        // everywhere, so this is what actually reskins every page (including
        // ones not yet rewritten to the new style) to the single dark-teal
        // palette, instead of only affecting components that opt in.
        slate: {
          50: '#f3f6f9',
          100: '#e6edf3',
          200: '#c9d6e0',
          300: '#9fb0c0',
          400: '#5b6b7a',
          500: '#3d4652',
          600: '#2b333e',
          700: '#1e2732',
          800: '#161b22',
          900: '#0d1117',
          950: '#080a0e',
        },
        emerald: {
          300: '#96e9c8',
          400: '#4fd1a5',
          500: '#22a67e',
          900: '#0f5540',
          950: '#0c4433',
        },
      },
      fontFamily: {
        sans: ['"Space Grotesk"', 'system-ui', '-apple-system', 'sans-serif'],
        mono: ['"IBM Plex Mono"', 'Menlo', 'Monaco', 'Courier New', 'monospace'],
      },
    },
  },
  plugins: [],
}
