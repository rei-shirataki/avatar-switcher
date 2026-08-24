const { config, unstable_createTailwindConfigTokenV2 } = require('@charcoal-ui/tailwind-config');

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
  // v1(config)はch-focus-ring等のプラグイン、v2(unstable_createTailwindConfigTokenV2)は
  // 実際にアプリで使っているv2トークン(--charcoal-color-*)に紐づくcolors/borderRadius等を提供する。
  // 後勝ちなのでv2を後に置き、colors/borderRadiusはv2の値を優先させる。
  presets: [config, unstable_createTailwindConfigTokenV2()],
};
