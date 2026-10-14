import console from 'node:console';
import { options } from './contracts.mjs';
import { runComposition } from './composition.mjs';

const args = options(['prepared', 'out', 'timeout-ms']);
console.log(JSON.stringify(await runComposition({
  prepared: args.prepared, out: args.out,
  timeoutMs: args['timeout-ms'] === undefined ? 60_000 : Number(args['timeout-ms']),
}), null, 2));
