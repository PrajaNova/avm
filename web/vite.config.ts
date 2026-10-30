import { defineConfig } from 'vite';
import { reactRouter } from '@react-router/dev/vite';

export default defineConfig({
  plugins: [reactRouter()],
  // The site reads the plugin registry from the repo root, one level above this project.
  server: { fs: { allow: ['..'] } },
});
