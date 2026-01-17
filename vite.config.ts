import path from 'path';
import { defineConfig, loadEnv } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig(({ mode }) => {
    const env = loadEnv(mode, '.', '');
    return {
      server: {
        port: 3000,
        host: '0.0.0.0',
        strictPort: true,
      },
      clearScreen: false,
      plugins: [svelte()],
      define: {
        'process.env.API_KEY': JSON.stringify(env.GEMINI_API_KEY),
        'process.env.GEMINI_API_KEY': JSON.stringify(env.GEMINI_API_KEY)
      },
      resolve: {
        alias: {
          '@': path.resolve(__dirname, 'src'),
          '$lib': path.resolve(__dirname, 'src/lib'),
          '$features': path.resolve(__dirname, 'src/features'),
          '$shared': path.resolve(__dirname, 'src/shared'),
        }
      }
    };
});
