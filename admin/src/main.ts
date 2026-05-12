import { createApp } from 'vue'
import { createPinia } from 'pinia'
import { FontAwesomeIcon } from '@fortawesome/vue-fontawesome'
import router from './router'
import { i18n } from './i18n'
import App from './App.vue'
import './icons'
import './style.css'

const app = createApp(App)
app.use(createPinia())
app.use(router)
app.use(i18n)
app.component('FontAwesomeIcon', FontAwesomeIcon)
app.mount('#app')
