import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    // Dev proxy: izbjegava CORS probleme tokom lokalnog razvoja.
    // Frontend na :5173 ce proslijedivati /api/* pozive na api_gateway na :8006.
    // U produkciji (nginx) konfigurisi proxy na web serveru.
    proxy: {
      '/api': {
        target: 'http://localhost:8006',
        changeOrigin: true,
        secure: false,
      },
    },
  },
});
