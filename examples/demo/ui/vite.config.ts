import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))

// https://vite.dev/config/
//
// `pnpm dev`  → Vite HMR for Bevy (ViteDevSource)
// `pnpm build` → single ESM bundle at dist/app.js (AssetServer / embed)
export default defineConfig(({ mode }) => ({
  plugins: [react()],
  resolve: {
    dedupe: ['react'],
  },
  define: {
    'process.env.NODE_ENV': JSON.stringify(
      mode === 'production' ? 'production' : 'development',
    ),
  },
  build: {
    lib: {
      entry: path.resolve(__dirname, 'src/main.tsx'),
      formats: ['es'],
      fileName: () => 'app.js',
    },
    rollupOptions: {
      external: [],
    },
    target: 'esnext',
    minify: mode === 'production',
    sourcemap: true,
    outDir: 'dist',
    emptyOutDir: true,
  },
}))
