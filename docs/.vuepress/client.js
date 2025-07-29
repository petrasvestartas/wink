import { defineClientConfig } from '@vuepress/client'
import WinkDemo from './components/WinkDemo.vue'

export default defineClientConfig({
  enhance({ app, router, siteData }) {
    // Register components globally
    app.component('WinkDemo', WinkDemo)
  },
})
