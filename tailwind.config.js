const { config } = require('@charcoal-ui/tailwind-config');

/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    './src/**/*.{html,ts}',
    './projects/overlay-ui/src/**/*.{html,ts}',
  ],
  theme: {
    extend: {},
  },
  plugins: [],
  presets: [config],
};
