import { paraglideVitePlugin } from '@inlang/paraglide-js';
import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [
    tailwindcss(),
    paraglideVitePlugin({
      project: './project.inlang',
      outdir: './src/lib/paraglide',
      // Desktop/mobile webview: no URL routing, remembered choice then system language.
      strategy: ['localStorage', 'preferredLanguage', 'baseLocale']
    }),
    sveltekit({
      preprocess: vitePreprocess(),
      adapter: adapter({ fallback: 'index.html' }),
      alias: { $lib: 'src/lib' }
    })
  ],
  clearScreen: false,
  server: { port: 1420, strictPort: true, host: '127.0.0.1' }
});
