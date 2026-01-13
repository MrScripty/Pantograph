import { mount } from 'svelte';
import App from './App.svelte';
import './styles.css';

const target = document.getElementById('app');
if (!target) {
  throw new Error('Could not find root element to mount to');
}

mount(App, { target });
