import { mount } from 'svelte';
import App from './App.svelte';
import './styles.css';

const target = document.getElementById('app');
if (!target) {
  const error = 'Could not find root element to mount to';
  console.error('[Pantograph] Mount error:', error);
  const errorContainer = document.createElement('div');
  errorContainer.style.padding = '20px';
  errorContainer.style.color = 'red';
  errorContainer.style.fontFamily = 'monospace';
  errorContainer.textContent = error;
  document.body.replaceChildren(errorContainer);
  throw new Error(error);
}

try {
  mount(App, { target });
} catch (error) {
  console.error('[Pantograph] Failed to mount application:', error);
  const container = document.createElement('div');
  container.style.padding = '20px';
  container.style.fontFamily = 'monospace';

  const title = document.createElement('h2');
  title.style.color = 'red';
  title.textContent = 'Application failed to start';

  const pre = document.createElement('pre');
  pre.style.background = '#f5f5f5';
  pre.style.padding = '10px';
  pre.style.overflow = 'auto';
  pre.textContent = error instanceof Error ? error.stack || error.message : String(error);

  const hint = document.createElement('p');
  hint.textContent = 'Check the browser console for more details.';

  container.append(title, pre, hint);
  target.replaceChildren(container);
  throw error;
}
