/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        'higashi-concrete': {
          100: '#F7F8F9',
          200: '#F0F1F2',
          300: '#D3D4D5',
          400: '#B6B7B8',
          500: '#999A9B',
          600: '#7C7D7E',
          700: '#606061',
          800: '#434343',
          900: '#272727',
        },
        'higashi-kashmirblue': {
          100: '#7FB3E4',
          200: '#6F9CC7',
          300: '#5F85A9',
          400: '#4F6F8C',
          500: '#3D576F',
          600: '#2F4152',
          700: '#1E2A35',
          800: '#152029',
          900: '#121A21',
        },
      },
    },
  },
  plugins: [],
};
