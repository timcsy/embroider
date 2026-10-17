import { mount } from 'svelte'
import { registerSW } from 'virtual:pwa-register'
import './app.css'
import App from './App.svelte'

// 正式版才註冊 service worker（開發時不要，免得快取擋住改動）
if (import.meta.env.PROD) registerSW({ immediate: true })

const app = mount(App, {
  target: document.getElementById('app')!,
})

export default app
