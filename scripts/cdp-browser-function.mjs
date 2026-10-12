// The callback is authored code. Runtime/DOM values travel as CDP arguments,
// never as interpolated JavaScript source or as a sanitized code string.
export async function evaluateBrowserFunction(call, callback, args = []) {
  if (typeof callback !== 'function') throw new TypeError('Browser callback must be a function');
  const context = await call('Runtime.evaluate', { expression: 'globalThis', returnByValue: false });
  if (context.exceptionDetails || !context.result?.objectId) throw new Error('Browser execution context unavailable');
  const objectId = context.result.objectId;
  try {
    const result = await call('Runtime.callFunctionOn', {
      objectId, functionDeclaration: Function.prototype.toString.call(callback),
      arguments: args.map(value => ({ value })), awaitPromise: true, returnByValue: true,
    });
    if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
    return result.result.value;
  } finally {
    await call('Runtime.releaseObject', { objectId });
  }
}
