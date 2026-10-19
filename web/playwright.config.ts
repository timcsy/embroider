import { defineConfig, devices } from '@playwright/test'

// 手機視窗（Pixel 7）模擬，主要使用者用 Android Chrome。
export default defineConfig({
  testDir: './tests/e2e',
  timeout: 60_000,
  // 在 CI 上機器比較慢、偶爾會有一次性的逾時，失敗時多試一次
  retries: process.env.CI ? 1 : 0,
  use: {
    baseURL: 'http://localhost:4173',
    ...devices['Pixel 7'],
  },
  webServer: {
    command: 'npm run build && npm run preview -- --port 4173 --strictPort',
    url: 'http://localhost:4173',
    reuseExistingServer: true,
    timeout: 180_000,
  },
})
