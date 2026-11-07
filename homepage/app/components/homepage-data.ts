import { family } from 'ferramenta-family/registry';

import samples from '../data/diagram-samples.json';

/* Facts the homepage's chapters share: the registry entry and the version of
   the package that drew every specimen. */

const member = family.find((entry) => entry.name === 'ferrigura');
if (member === undefined) throw new Error('Ferrigura is missing from the family registry.');
export const tool = member;

export const core = samples.renderedWith['@ferrigura/core'];

export const grouped = (value: number) => value.toLocaleString('en-US');
