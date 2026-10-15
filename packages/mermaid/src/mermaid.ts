/**
 * Web page integration module for the mermaid framework. It uses the mermaidAPI for mermaid
 * functionality and to render the diagrams to svg code!
 */
import type { AsyncIconLoader, IconLoader, SyncIconLoader } from './rendering-util/icons.js';
import { registerIconPacks } from './rendering-util/icons.js';
import { dedent } from 'ts-dedent';
import type { MermaidConfig } from './config.type.js';
import { detectType, detectors, registerLazyLoadedDiagrams } from './diagram-api/detectType.js';
import { addDiagrams } from './diagram-api/diagram-orchestration.js';
import { loadRegisteredDiagrams } from './diagram-api/loadDiagram.js';
import type { ExternalDiagramDefinition, SVG, SVGGroup } from './diagram-api/types.js';
import type { ParseErrorFunction } from './Diagram.js';
import type { UnknownDiagramError } from './errors.js';
import type { InternalHelpers } from './internals.js';
import { log } from './logger.js';
import { mermaidAPI } from './mermaidAPI.js';
import type { LayoutLoaderDefinition, RenderOptions } from './rendering-util/render.js';
import { registerLayoutLoaders } from './rendering-util/render.js';
import type { LayoutData } from './rendering-util/types.js';
import type { ParseOptions, ParseResult, RenderResult } from './types.js';
import type { DetailedError } from './utils.js';
import utils, { isDetailedError } from './utils.js';

export type {
  AsyncIconLoader,
  DetailedError,
  ExternalDiagramDefinition,
  IconLoader,
  InternalHelpers,
  LayoutData,
  LayoutLoaderDefinition,
  MermaidConfig,
  ParseErrorFunction,
  ParseOptions,
  ParseResult,
  RenderOptions,
  RenderResult,
  SVG,
  SVGGroup,
  SyncIconLoader,
  UnknownDiagramError,
};

export interface RunOptions {
  /**
   * The query selector to use when finding elements to render. Default: `".mermaid"`.
   */
  querySelector?: string;
  /**
   * The nodes to render. If this is set, `querySelector` will be ignored.
   */
  nodes?: ArrayLike<HTMLElement>;
  /**
   * A callback to call after each diagram is rendered.
   */
  postRenderCallback?: (id: string) => unknown;
  /**
   * If `true`, errors will be logged to the console, but not thrown. Default: `false`
   */
  suppressErrors?: boolean;
}

const handleError = (error: unknown, errors: DetailedError[], parseError?: ParseErrorFunction) => {
  log.warn(error);
  if (isDetailedError(error)) {
    // handle case where error string and hash were
    // wrapped in object like`const error = { str, hash };`
    if (parseError) {
      parseError(error.str, error.hash);
    }
    errors.push({ ...error, message: error.str, error });
  } else {
    // assume it is just error string and pass it on
    if (parseError) {
      parseError(error);
    }
    if (error instanceof Error) {
      errors.push({
        str: error.message,
        message: error.message,
        hash: error.name,
        error,
      });
    }
  }
};

/**
 * ## run
 *
 * Function that goes through the document to find the chart definitions in there and render them.
 *
 * The function tags the processed attributes with the attribute data-processed and ignores found
 * elements with the attribute already set. This way the run function can be triggered several
 * times.
 *
 * ```mermaid
 * graph LR;
 *  a(Find elements)-->b{Processed}
 *  b-->|Yes|c(Leave element)
 *  b-->|No |d(Transform)
 * ```
 *
 * Renders the mermaid diagrams
 *
 * @param options - Optional runtime configs
 */
const run = async function (
  options: RunOptions = {
    querySelector: '.mermaid',
  }
) {
  try {
    await runThrowsErrors(options);
  } catch (e) {
    if (isDetailedError(e)) {
      log.error(e.str);
    }
    if (mermaid.parseError) {
      mermaid.parseError(e as string);
    }
    if (!options.suppressErrors) {
      log.error('Use the suppressErrors option to suppress these errors');
      throw e;
    }
  }
};

const runThrowsErrors = async function (
  { postRenderCallback, querySelector, nodes }: Omit<RunOptions, 'suppressErrors'> = {
    querySelector: '.mermaid',
  }
) {
  const conf = mermaidAPI.getConfig();

  log.debug(`${!postRenderCallback ? 'No ' : ''}Callback function found`);

  let nodesToProcess: ArrayLike<HTMLElement>;
  if (nodes) {
    nodesToProcess = nodes;
  } else if (querySelector) {
    nodesToProcess = document.querySelectorAll(querySelector);
  } else {
    throw new Error('Nodes and querySelector are both undefined');
  }

  log.debug(`Found ${nodesToProcess.length} diagrams`);

  // generate the id of the diagram
  const idGenerator = new utils.InitIDGenerator(conf.deterministicIds, conf.deterministicIDSeed);

  let txt: string;
  const errors: DetailedError[] = [];

  // element is the current div with mermaid class
  // eslint-disable-next-line unicorn/prefer-spread
  for (const element of Array.from(nodesToProcess)) {
    log.info('Rendering diagram: ' + element.id);
    /*! Check if previously processed */
    if (element.getAttribute('data-processed')) {
      continue;
    }
    element.setAttribute('data-processed', 'true');

    const id = `mermaid-${idGenerator.next()}`;

    // Fetch the graph definition including tags
    txt = element.innerHTML;

    // transforms the html to pure text
    txt = dedent(utils.entityDecode(txt)) // removes indentation, required for YAML parsing
      .trim()
      .replace(/<br\s*\/?>/gi, '<br/>');

    try {
      const { svg, bindFunctions } = await render(id, txt, element);
      element.innerHTML = svg;
      if (postRenderCallback) {
        await postRenderCallback(id);
      }
      if (bindFunctions) {
        bindFunctions(element);
      }
    } catch (error) {
      handleError(error, errors, mermaid.parseError);
    }
  }
  if (errors.length > 0) {
    // TODO: We should be throwing an error object.
    throw errors[0];
  }
};

/**
 * Used to set configurations for mermaid.
 * This function should be called before the run function.
 * @param config - Configuration object for mermaid.
 */

const initialize = function (config: MermaidConfig) {
  mermaidAPI.initialize(config);
};

/**
 * Used to register external diagram types.
 * @param diagrams - Array of {@link ExternalDiagramDefinition}.
 * @param opts - If opts.lazyLoad is false, the diagrams will be loaded immediately.
 */
const registerExternalDiagrams = async (
  diagrams: ExternalDiagramDefinition[],
  {
    lazyLoad = true,
  }: {
    lazyLoad?: boolean;
  } = {}
) => {
  addDiagrams();
  registerLazyLoadedDiagrams(...diagrams);
  if (lazyLoad === false) {
    await loadRegisteredDiagrams();
  }
};

/**
 * ##contentLoaded Callback function that is called when page is loaded. If the `startOnLoad`
 * configuration is enabled, it calls {@link run} to render the mermaid diagrams on the page.
 */
const contentLoaded = function () {
  const { startOnLoad } = mermaidAPI.getConfig();
  if (startOnLoad) {
    mermaid.run().catch((err) => log.error('Mermaid failed to initialize', err));
  }
};

if (typeof document !== 'undefined') {
  /*!
   * Wait for document loaded before starting the execution
   */
  window.addEventListener('load', contentLoaded, false);
}

const executionQueue: (() => Promise<unknown>)[] = [];
let executionQueueRunning = false;
const executeQueue = async () => {
  if (executionQueueRunning) {
    return;
  }
  executionQueueRunning = true;
  while (executionQueue.length > 0) {
    const f = executionQueue.shift();
    if (f) {
      try {
        await f();
      } catch (e) {
        log.error('Error executing queue', e);
      }
    }
  }
  executionQueueRunning = false;
};

/**
 * Parse the text and validate the syntax.
 * @param text - The mermaid diagram definition.
 * @param parseOptions - Options for parsing. @see {@link ParseOptions}
 * @returns If valid, {@link ParseResult} otherwise `false` if parseOptions.suppressErrors is `true`.
 * @throws Error if the diagram is invalid and parseOptions.suppressErrors is false or not set.
 *
 * @example
 * ```js
 * console.log(await mermaid.parse('flowchart \n a --> b'));
 * // { diagramType: 'flowchart-v2' }
 * console.log(await mermaid.parse('wrong \n a --> b', { suppressErrors: true }));
 * // false
 * console.log(await mermaid.parse('wrong \n a --> b', { suppressErrors: false }));
 * // throws Error
 * console.log(await mermaid.parse('wrong \n a --> b'));
 * // throws Error
 * ```
 */
const parse: typeof mermaidAPI.parse = async (text, parseOptions) => {
  return new Promise((resolve, reject) => {
    // This promise will resolve when the render call is done.
    // It will be queued first and will be executed when it is first in line
    const performCall = () =>
      new Promise((res, rej) => {
        mermaidAPI.parse(text, parseOptions).then(
          (r) => {
            // This resolves for the promise for the queue handling
            res(r);
            // This fulfills the promise sent to the value back to the original caller
            resolve(r);
          },
          (e) => {
            log.error('Error parsing', e);
            mermaid.parseError?.(e);
            rej(e);
            reject(e);
          }
        );
      });
    executionQueue.push(performCall);
    executeQueue().catch(reject);
  });
};

/**
 * Function that renders an SVG with a graph from a chart definition. Usage example below.
 *
 * ```javascript
 *  element = document.querySelector('#graphDiv');
 *  const graphDefinition = 'graph TB\na-->b';
 *  const { svg, bindFunctions } = await mermaid.render('graphDiv', graphDefinition);
 *  element.innerHTML = svg;
 *  bindFunctions?.(element);
 * ```
 *
 * @remarks
 * Multiple calls to this function will be enqueued to run serially.
 *
 * @param id - The id for the SVG element (the element to be rendered)
 * @param text - The text for the graph definition
 * @param container - HTML element where the svg will be inserted. (Is usually element with the .mermaid class)
 *   If no svgContainingElement is provided then the SVG element will be appended to the body.
 *    Selector to element in which a div with the graph temporarily will be
 *   inserted. If one is provided a hidden div will be inserted in the body of the page instead. The
 *   element will be removed when rendering is completed.
 * @returns Returns the SVG Definition and BindFunctions.
 */
const render: typeof mermaidAPI.render = (id, text, container) => {
  return new Promise((resolve, reject) => {
    // This promise will resolve when the mermaidAPI.render call is done.
    // It will be queued first and will be executed when it is first in line
    const performCall = () =>
      new Promise((res, rej) => {
        mermaidAPI.render(id, text, container).then(
          (r) => {
            // This resolves for the promise for the queue handling
            res(r);
            // This fulfills the promise sent to the value back to the original caller
            resolve(r);
          },
          (e) => {
            log.error('Error parsing', e);
            mermaid.parseError?.(e);
            rej(e);
            reject(e);
          }
        );
      });
    executionQueue.push(performCall);
    executeQueue().catch(reject);
  });
};

/**
 * Gets the metadata for all registered diagrams.
 * Currently only the id is returned.
 * @returns An array of objects with the id of the diagram.
 */
const getRegisteredDiagramsMetadata = (): Pick<ExternalDiagramDefinition, 'id'>[] => {
  return Object.keys(detectors).map((id) => ({
    id,
  }));
};

export interface Mermaid {
  parseError?: ParseErrorFunction;
  parse: typeof parse;
  render: typeof render;
  run: typeof run;
  registerLayoutLoaders: typeof registerLayoutLoaders;
  registerExternalDiagrams: typeof registerExternalDiagrams;
  initialize: typeof initialize;
  contentLoaded: typeof contentLoaded;
  detectType: typeof detectType;
  registerIconPacks: typeof registerIconPacks;
  getRegisteredDiagramsMetadata: typeof getRegisteredDiagramsMetadata;
}

const mermaid: Mermaid = {
  parse,
  render,
  run,
  registerExternalDiagrams,
  registerLayoutLoaders,
  initialize,
  parseError: undefined,
  contentLoaded,
  detectType,
  registerIconPacks,
  getRegisteredDiagramsMetadata,
};

export default mermaid;
