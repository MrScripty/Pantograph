import console from 'node:console';
import { options } from './contracts.mjs';
import { inspectBundle } from './composition.mjs';

const args = options(['prepared', 'out']);
console.log(JSON.stringify(await inspectBundle(args), null, 2));
