import path from 'path';
import { defineConfig, loadEnv, Plugin } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

// Pattern for generated components - used to split Svelte plugin instances
const GENERATED_PATTERN = 'src/generated/**/*.svelte';

/**
 * Vite plugin to handle HMR for generated components.
 * Prevents full page reload when files are deleted/created during undo operations.
 * Works in conjunction with a separate Svelte plugin instance that has hot: false.
 */
function generatedComponentsHMR(): Plugin {
  return {
    name: 'generated-components-hmr',
    enforce: 'pre', // Run before other plugins including Svelte

    hotUpdate({ type, file, server }) {
      // Only handle files in src/generated/
      if (!file.includes('/src/generated/')) {
        return; // Let Vite handle normally
      }

      // Skip validation temp files - these are created/deleted during component validation
      // and should not trigger HMR events
      if (file.includes('.validate-') || file.endsWith('.tmp')) {
        return; // Ignore temp files
      }

      const eventMap: Record<string, string> = {
        'delete': 'generated-component-deleted',
        'create': 'generated-component-created',
        'update': 'generated-component-updated'
      };

      const event = eventMap[type];
      if (event) {
        // Handle module graph operations
        if (type === 'delete') {
          server.moduleGraph.onFileDelete(file);
        } else if (type === 'update') {
          // Invalidate module so next import gets fresh content
          const module = server.moduleGraph.getModuleById(file);
          if (module) {
            server.moduleGraph.invalidateModule(module);
          }
        }

        // Normalize path to URL-style for frontend (Vite provides absolute filesystem paths)
        const relativePath = '/src/generated/' + file.split('/src/generated/')[1];

        // Send custom event to frontend - we handle HMR ourselves
        // Check if WebSocket is available and ready before sending
        try {
          if (server.ws) {
            server.ws.send({
              type: 'custom',
              event,
              data: { file: relativePath }
            });
          }
        } catch (err) {
          // Ignore WebSocket errors (connection closed, EPIPE, etc.)
          // This can happen during app shutdown or HMR reconnection
          console.debug(`[generated-components-hmr] WebSocket send failed (likely shutting down):`, err);
        }

        console.log(`[generated-components-hmr] ${type}:`, relativePath);

        // Return empty array to prevent default HMR and stop propagation to Svelte plugin
        return [];
      }
    }
  };
}

export default defineConfig(({ mode }) => {
    const env = loadEnv(mode, '.', '');
    return {
      server: {
        port: 3000,
        host: '0.0.0.0',
        strictPort: true,
        watch: {
          // Ignore Rust build artifacts - target/doc alone is 2.2GB with thousands of files
          ignored: ['**/src-tauri/target/**']
        }
      },
      clearScreen: false,
      plugins: [
        generatedComponentsHMR(), // Must be before svelte() to intercept first
        // Normal Svelte with HMR - excludes generated components
        svelte({ exclude: GENERATED_PATTERN }),
        // Generated components - compiled but NO HMR (we handle it ourselves via custom events)
        svelte({
          include: GENERATED_PATTERN,
          hot: false,
          configFile: false // Don't read svelte.config.js twice
        })
      ],
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
      },
      optimizeDeps: {
        include: [
          // Icon library used by generated components
          'lucide-svelte',
          // Svelte internals
          'svelte',
          'svelte/animate',
          'svelte/motion',
          'svelte/store',
          'svelte/transition',
          // Other commonly used deps
          '@tauri-apps/api/core',
          '@tauri-apps/plugin-dialog',
          // Note: @xyflow/svelte has a known Svelte 5 compatibility issue:
          // The package ships pre-compiled .svelte.js files that use 'import * as $'
          // which conflicts with Svelte 5's reserved $ prefix.
          // See: https://github.com/xyflow/xyflow/issues/5200
          // '@xyflow/svelte',
        ],
      }
    };
});
