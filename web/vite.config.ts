/// <reference types="vitest/config" />
import { execSync } from 'node:child_process'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import { defineConfig } from 'vite'
import { VitePWA } from 'vite-plugin-pwa'
import { FONT_FILES } from './src/lib/fonts.ts'

// 版本編號：建置的時間加上程式碼的版本（沒有 git 就只有時間），畫面上看得到，才分得出現在開的是不是最新的
function buildId(): string {
  let hash = ''
  try {
    hash = execSync('git rev-parse --short HEAD', { stdio: ['ignore', 'pipe', 'ignore'] }).toString().trim()
  } catch {
    // 沒有 git 也沒關係
  }
  const d = new Date(Date.now() + 8 * 3600_000).toISOString().slice(0, 16).replace('T', ' ')
  return hash ? `${d} · ${hash}` : d
}

export default defineConfig({
  define: { __BUILD_ID__: JSON.stringify(buildId()) },
  // 相對路徑：放在任何子路徑（例如 GitHub Pages）都能運作
  base: './',
  plugins: [
    svelte(),
    // 離線與「加到主畫面」：把程式、WASM、字型都預先快取，之後沒有網路也能用
    VitePWA({
      // 不用 autoUpdate：它會在新版啟用時強制重新整理，使用者正在做的設計（選好的圖、打的字）會全部消失。
      // 新版會等到使用者下次重新打開才接手。
      registerType: 'prompt',
      injectRegister: false,
      manifest: {
        name: '刺繡小幫手',
        short_name: '刺繡',
        description: '在手機上打字或選圖，轉成可以給縫紉機繡的檔案。',
        lang: 'zh-TW',
        display: 'standalone',
        orientation: 'portrait',
        start_url: './',
        scope: './',
        background_color: '#fbf7ef',
        theme_color: '#b3282d',
        icons: [
          { src: './icons/icon-192.png', sizes: '192x192', type: 'image/png' },
          { src: './icons/icon-512.png', sizes: '512x512', type: 'image/png' },
          { src: './icons/icon-maskable-512.png', sizes: '512x512', type: 'image/png', purpose: 'maskable' },
        ],
      },
      workbox: {
        globPatterns: ['**/*.{js,css,html,wasm,svg,png,ttf,webmanifest}'],
        // 只有基本字型跟著程式預先快取；其他字型第一次用到才下載，下載後由下面的規則留住，之後離線也能用
        globIgnores: FONT_FILES.filter((f) => !f.preload).map((f) => `**/fonts/${f.file}`),
        runtimeCaching: [
          {
            urlPattern: /\.ttf$/,
            handler: 'CacheFirst',
            options: { cacheName: 'fonts-on-demand', cacheableResponse: { statuses: [0, 200] } },
          },
        ],
        maximumFileSizeToCacheInBytes: 6 * 1024 * 1024,
        navigateFallback: 'index.html',
      },
    }),
  ],
  test: {
    environment: 'jsdom',
    include: ['tests/**/*.test.ts'],
    exclude: ['tests/e2e/**', 'node_modules/**'],
  },
})
