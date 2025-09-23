/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{ts,tsx,js,jsx}",
    "./components/**/*.{ts,tsx,js,jsx}",
  ],
  theme: {
    extend: {
      fontFamily: {
        sans: ['Inter', 'ui-sans-serif', 'system-ui', 'sans-serif'],
        mono: ['JetBrains Mono', 'ui-monospace', 'SFMono-Regular', 'monospace'],
      },
      screens: {
        md: '768px',
      },
      spacing: {
        '1_5': '0.375rem',
        '2_5': '0.625rem',
        '3_5': '0.875rem',
        '4_5': '1.125rem',
        '5_5': '1.375rem',
        '6_5': '1.625rem',
        '7_5': '1.875rem',
        '8_5': '2.125rem',
        '9_5': '2.375rem',
        '10_5': '2.625rem',
        '11_5': '2.875rem',
        '12_5': '3.125rem',
        '13_5': '3.375rem',
        '14_5': '3.625rem',
        '15_5': '3.875rem',
      },
      gridTemplateColumns: {
        'sidebar': '260px 1fr',
        'sidebar-collapsed': '64px 1fr',
        'stats-1': 'repeat(1, minmax(0, 1fr))',
        'stats-2': 'repeat(2, minmax(0, 1fr))',
        'stats-3': 'repeat(3, minmax(0, 1fr))',
        'stats-4': 'repeat(4, minmax(0, 1fr))',
        'content-1': '1fr',
        'content-2': 'repeat(2, minmax(0, 1fr))',
        'content-3': 'repeat(3, minmax(0, 1fr))',
        'content-4': 'repeat(4, minmax(0, 1fr))',
        'content-5': 'repeat(5, minmax(0, 1fr))',
        'content-6': 'repeat(6, minmax(0, 1fr))',
      },
      gridColumn: {
        'span-13': 'span 13 / span 13',
        'span-14': 'span 14 / span 14',
        'span-15': 'span 15 / span 15',
        'span-16': 'span 16 / span 16',
      },
      animation: {
        'fade-in': 'fade-in 200ms ease-out',
        'fade-in-up': 'fade-in-up 0.3s ease-out forwards',
      },
      fontSize: {
        'xs': ['0.75rem', { lineHeight: '1.5', fontWeight: '400' }],
        'sm': ['0.875rem', { lineHeight: '1.5', fontWeight: '400' }],
        'base': ['1rem', { lineHeight: '1.6', fontWeight: '400' }],
        'lg': ['1.125rem', { lineHeight: '1.6', fontWeight: '500' }],
        'xl': ['1.25rem', { lineHeight: '1.5', fontWeight: '600' }],
        '2xl': ['1.5rem', { lineHeight: '1.4', fontWeight: '600' }],
        '3xl': ['2rem', { lineHeight: '1.33', fontWeight: '700' }],
      },
      keyframes: {
        'fade-in': {
          from: { opacity: '0' },
          to: { opacity: '1' }
        },
        'fade-in-up': {
          '0%': { opacity: '0', transform: 'translateY(10px)' },
          '100%': { opacity: '1', transform: 'translateY(0)' }
        }
      },
      colors: {
        // Brand colors
        brand: {
          50: 'hsl(24 100% 97%)',
          100: 'hsl(24 95% 92%)',
          200: 'hsl(24 90% 84%)',
          300: 'hsl(24 85% 70%)',
          400: 'hsl(24 85% 55%)',
          500: 'hsl(24 86% 42%)',
          600: 'hsl(24 88% 36%)',
          700: 'hsl(24 92% 30%)',
          800: 'hsl(24 95% 24%)',
          900: 'hsl(24 98% 18%)',
        },
        // Status colors
        success: {
          50: 'hsl(142 100% 97%)',
          100: 'hsl(142 90% 92%)',
          500: 'hsl(142 76% 36%)',
          600: 'hsl(142 78% 32%)',
        },
        warning: {
          50: 'hsl(42 100% 96%)',
          100: 'hsl(42 95% 90%)',
          500: 'hsl(42 96% 50%)',
          600: 'hsl(42 98% 45%)',
        },
        danger: {
          50: 'hsl(0 100% 97%)',
          100: 'hsl(0 95% 92%)',
          500: 'hsl(0 84% 50%)',
          600: 'hsl(0 86% 45%)',
        },
        info: {
          50: 'hsl(217 100% 97%)',
          100: 'hsl(217 95% 92%)',
          500: 'hsl(217 91% 60%)',
          600: 'hsl(217 93% 55%)',
        },
        // Background hierarchy
        bg: {
          primary: 'hsl(0 0% 100%)',
          secondary: 'hsl(210 40% 98%)',
          tertiary: 'hsl(210 40% 96%)',
          overlay: 'hsl(0 0% 0% / 0.5)',
        },
        // Text hierarchy
        text: {
          primary: 'hsl(220 15% 15%)',
          secondary: 'hsl(220 10% 40%)',
          tertiary: 'hsl(220 10% 55%)',
          inverse: 'hsl(0 0% 100%)',
        },
      },
    },
  },
  darkMode: 'class',
}
