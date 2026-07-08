/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        // Backgrounds
        background: '#0a0a0a',
        surface: '#171717',
        surfaceHighlight: '#262626',

        // Text
        foreground: '#f5f5f5',
        muted: '#737373',

        // Brand
        primary: '#3b82f6',
        primaryHover: '#2563eb',
        primaryForeground: '#ffffff',

        // Semantic
        success: '#22c55e',
        warning: '#f59e0b',
        danger: '#ef4444',
        info: '#38bdf8',
      },
      fontFamily: {
        mono: ["'JetBrains Mono'", "'Fira Code'", 'monospace'],
      },
    },
  },
  plugins: [],
};
