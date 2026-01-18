import { mount } from 'svelte';
import App from './App.svelte';
import './styles.css';

const target = document.getElementById('app');
if (!target) {
  const error = 'Could not find root element to mount to';
  console.error('[Pantograph] Mount error:', error);
  document.body.innerHTML = `<div style="padding: 20px; color: red; font-family: monospace;">${error}</div>`;
  throw new Error(error);
}

try {
  mount(App, { target });
} catch (error) {
  console.error('[Pantograph] Failed to mount application:', error);
  target.innerHTML = `
    <div style="padding: 20px; font-family: monospace;">
      <h2 style="color: red;">Application failed to start</h2>
      <pre style="background: #f5f5f5; padding: 10px; overflow: auto;">${error instanceof Error ? error.stack || error.message : String(error)}</pre>
      <p>Check the browser console for more details.</p>
    </div>
  `;
  throw error;
}
