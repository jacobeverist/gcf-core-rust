import { defineConfig } from 'vite';
import { copyFileSync, mkdirSync, existsSync, readdirSync, statSync } from 'fs';
import { join } from 'path';

// Basic Vite config suitable for developing the TS migration alongside existing HTML.
export default defineConfig({
  root: '.',
  publicDir: false, // Disable publicDir to allow imports from pkg/
  server: {
    port: 5173,
    open: false,
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    rollupOptions: {
      output: {
        assetFileNames: (assetInfo) => {
          // Keep pkg files in their original structure
          if (assetInfo.name?.startsWith('pkg/')) {
            return assetInfo.name;
          }
          return 'assets/[name]-[hash][extname]';
        },
      },
    },
  },
  assetsInclude: ['**/*.wasm'],
  plugins: [
    {
      name: 'copy-pkg',
      closeBundle() {
        const pkgSrc = 'pkg';
        const pkgDest = 'dist/pkg';
        
        if (existsSync(pkgSrc)) {
          if (!existsSync(pkgDest)) {
            mkdirSync(pkgDest, { recursive: true });
          }
          
          const copyRecursive = (src: string, dest: string) => {
            const entries = readdirSync(src);
            for (const entry of entries) {
              const srcPath = join(src, entry);
              const destPath = join(dest, entry);
              if (statSync(srcPath).isDirectory()) {
                if (!existsSync(destPath)) mkdirSync(destPath, { recursive: true });
                copyRecursive(srcPath, destPath);
              } else {
                copyFileSync(srcPath, destPath);
              }
            }
          };
          
          copyRecursive(pkgSrc, pkgDest);
          console.log('✓ Copied pkg/ to dist/pkg/');
        }
      },
    },
  ],
});
