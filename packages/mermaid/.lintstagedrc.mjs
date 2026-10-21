import baseConfig from '../../.lintstagedrc.mjs';
export default {
  ...baseConfig,
  'src/schemas/config.schema.yaml': ['pnpm --filter mermaid run types:build-config --git'],
};
