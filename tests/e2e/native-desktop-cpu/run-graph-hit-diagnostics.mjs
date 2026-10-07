// Read-only observation; selection always uses the WebDriver element click below it.
export function readRunGraphHitDiagnostics(node, target) {
  const describe = (element) => ({ tag: element.tagName.toLowerCase(),
    label: element.getAttribute('aria-label'), isNode: element === node,
    isTarget: element === target, inNode: node.contains(element) });
  const inspect = (element) => {
    const rects = Array.from(element.getClientRects(), (rect) => ({
      left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom,
      width: rect.width, height: rect.height,
    }));
    const rect = rects[0];
    if (!rect) return { ...describe(element), rects, center: null, hits: [] };
    const left = Math.max(0, rect.left);
    const right = Math.min(window.innerWidth, rect.right);
    const top = Math.max(0, rect.top);
    const bottom = Math.min(window.innerHeight, rect.bottom);
    if (left >= right || top >= bottom) return { ...describe(element), rects, center: null, hits: [] };
    const center = { x: Math.floor((left + right) / 2), y: Math.floor((top + bottom) / 2) };
    return { ...describe(element), rects, center,
      hits: document.elementsFromPoint(center.x, center.y).map(describe) };
  };
  return { node: inspect(node), target: inspect(target) };
}
