import { afterEach, describe, expect, it, vi } from 'vitest';
import { log, setLogLevel } from './logger.js';

describe('setLogLevel', () => {
  afterEach(() => {
    setLogLevel('fatal');
    vi.restoreAllMocks();
  });

  it('accepts a level name in any case', () => {
    const debug = vi.spyOn(console, 'debug').mockImplementation(() => undefined);
    setLogLevel('DEBUG' as 'debug');

    log.debug('visible');

    expect(debug).toHaveBeenCalledOnce();
  });

  it('falls back to fatal for a name that is not a level', () => {
    const error = vi.spyOn(console, 'error').mockImplementation(() => undefined);
    setLogLevel('constructor' as 'debug');

    log.error('hidden');
    log.fatal('visible');

    expect(error).toHaveBeenCalledOnce();
  });
});
