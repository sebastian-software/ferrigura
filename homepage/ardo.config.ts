import { defineArdoConfig } from 'ardo/config';

export default defineArdoConfig({
  title: 'Ferrigura',
  description:
    'Render Mermaid diagram text to standalone SVG in Rust, with bundled fonts and selectable labels. No browser process required.',
  base: '/',
  siteUrl: 'https://ferrigura.dev',
  icons: false,
  markdown: { headingAttributes: true },
  linkCheck: { level: 'error' },
});
