import { NATIVE_TARGETS } from 'ferramenta-native/targets';
import path from 'node:path';
const root = import.meta.dirname;
export default {
  name: 'ferrigura',
  packageDir: path.join(root, 'ferrigura'),
  platformDir: path.join(root, 'platforms'),
  packagePattern: '@ferrigura/{target}',
  binaryPattern: 'ferrigura.node',
  runtimeFile: 'native-runtime.mjs',
  repositoryPlatformDir: 'node/platforms',
  licenses: ['LICENSE', 'NOTICE.md'],
  workspaceProtocol: true,
  pack: { required: ['index.mjs', 'index.d.mts', 'native-runtime.mjs'], maxUnpackedSize: 200_000 },
  consumer: {
    smoke: `import { render } from '@ferrigura/core'; const result = render('graph TD; A-->B'); if (!result.includes('<svg')) throw new Error('SVG mismatch');`,
  },
  release: {
    repository: 'https://github.com/sebastian-software/ferrigura',
    tagPrefix: 'v',
    ci: {
      requiredJobs: ['lint', 'test', 'rust', 'snapshots', 'port-geometry'],
      platformJobs: NATIVE_TARGETS.map(({ id }) => ({
        pattern: `^native-npm \\(.*${id}`,
        label: `native-npm ${id}`,
      })),
    },
    cargoFiles: ['Cargo.toml', 'crates/ferrigura-node/Cargo.toml'],
  },
};
