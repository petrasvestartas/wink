import { defineUserConfig } from 'vuepress'
import { defaultTheme } from '@vuepress/theme-default'
import { viteBundler } from '@vuepress/bundler-vite'

export default defineUserConfig({
  lang: 'en-US',
  title: 'Wink - Rust WASM Demo',
  description: 'A Rust WASM application using winit and wgpu',
  
  theme: defaultTheme({
    navbar: [
      {
        text: 'Home',
        link: '/',
      },
      {
        text: 'Demo',
        link: '/demo/',
      },
    ],
  }),
  
  bundler: viteBundler(),
})