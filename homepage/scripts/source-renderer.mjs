/* eslint-disable security/detect-non-literal-fs-filename -- Configuration lives in an owned temporary directory. */
import { execFileSync } from 'node:child_process';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const repository = new URL('../../', import.meta.url);
const binary = fileURLToPath(new URL('target/debug/examples/render', repository));

/** The current Rust source, with one configuration shared by a set of drawings. */
export async function createSourceRenderer(config = {}) {
  execFileSync('cargo', ['build', '-q', '-p', 'ferrigura', '--example', 'render'], {
    cwd: repository,
    stdio: 'inherit',
  });
  const directory = await mkdtemp(join(tmpdir(), 'ferrigura-source-'));
  const remove = () => rm(directory, { recursive: true, force: true });
  const configPath = join(directory, 'config.json');
  try {
    await writeFile(configPath, JSON.stringify(config));
  } catch (error) {
    await remove();
    throw error;
  }
  return {
    remove,
    render(source, options = {}) {
      const args = ['--config', configPath, '--compat', String(options.compat ?? false)];
      if (options.id !== undefined) args.push('--id', options.id);
      if (options.colorScheme !== undefined) args.push('--scheme', options.colorScheme);
      return execFileSync(binary, args, {
        cwd: repository,
        input: source,
        encoding: 'utf8',
        maxBuffer: 16 * 1024 * 1024,
      });
    },
  };
}
