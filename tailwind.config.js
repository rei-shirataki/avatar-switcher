const { unstable_createTailwindConfigTokenV2 } = require('@charcoal-ui/tailwind-config');

const v2 = unstable_createTailwindConfigTokenV2();

/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    './src/**/*.{html,ts}',
    './projects/overlay-ui/src/**/*.{html,ts}',
  ],
  theme: {
    // v1のconfig(presets)はtheme.spacing/gap/widthをcharcoal独自スケールで完全上書きしてしまい、
    // p-4等の標準Tailwindクラスの意味が変わってしまう罠があるため使わない。
    // colors/borderRadiusだけをextendで注入し、spacing/gap/width/heightは標準スケールを維持する。
    extend: {
      colors: {
        ...v2.theme.colors,
        // avatar-switcher独自のブランド色。charcoal標準パレットに対応する色がないため専用トークンとして維持。
        primary: {
          DEFAULT: 'var(--color-primary)',
          hover: 'var(--color-primary-hover)',
          active: 'var(--color-primary-active)',
          dim: 'var(--color-primary-dim)',
        },
        // overlay-ui独自のアクセント色。同じくcharcoal標準パレットに対応する色がない。
        accent: {
          DEFAULT: '#2b6fd6',
          hover: '#3f82ea',
          label: '#9ad1ff',
        },
      },
      borderRadius: v2.theme.borderRadius,
    },
  },
  plugins: [],
};
