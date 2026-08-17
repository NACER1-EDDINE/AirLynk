/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        ink: {
          DEFAULT: '#E8ECF0',
          dark: '#E8ECF0',
          light: '#0E1116',
        },
        paper: {
          DEFAULT: '#FFFFFF',
          dark: '#FFFFFF',
          light: '#FFFFFF',
        },
        scrim: {
          DEFAULT: 'rgba(20, 23, 27, 0.78)',
          dark: 'rgba(20, 23, 27, 0.78)',
          light: 'rgba(242, 244, 246, 0.85)',
        },
        structure: {
          DEFAULT: 'rgba(139, 147, 156, 0.4)',
          dark: 'rgba(139, 147, 156, 0.40)',
          light: 'rgba(139, 147, 156, 0.45)',
        },
        signal: {
          DEFAULT: '#4C8DF6',
          dark: '#4C8DF6',
          light: '#1B5FC1',
        },
        void: {
          DEFAULT: '#E0574A',
          dark: '#E0574A',
          light: '#C0362C',
        },
      },
      fontFamily: {
        sans: ['"Segoe UI Variable"', '"Segoe UI"', 'system-ui', 'sans-serif'],
      },
      fontSize: {
        'band-label': ['11px', { letterSpacing: '0.14em', textTransform: 'uppercase', fontWeight: '400' }],
        'text-base': ['13px', { lineHeight: '1.4' }],
        'display': ['20px', { fontWeight: '600', letterSpacing: '0' }],
        'session-code': ['28px', { fontWeight: '600', letterSpacing: '0.08em' }],
      },
      borderWidth: {
        'hairline': '0.5px',
      },
      borderStyle: {
        'dashed': 'dashed',
      },
      borderRadius: {
        'circle': '50%',
        'qr': '0',
      },
      backgroundImage: {
        'scrim-dark': 'linear-gradient(rgba(20, 23, 27, 0.78), rgba(20, 23, 27, 0.78))',
        'scrim-light': 'linear-gradient(rgba(242, 244, 246, 0.85), rgba(242, 244, 246, 0.85))',
      },
    },
  },
  plugins: [],
}