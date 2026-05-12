import { createI18n } from 'vue-i18n'
import enUS from './en-US'
import zhCN from './zh-CN'

const savedLocale = localStorage.getItem('admin-locale') || 'en'

export const i18n = createI18n({
  legacy: false,
  locale: savedLocale,
  fallbackLocale: 'en',
  messages: {
    en: enUS,
    zh: zhCN,
  },
})

export type Locale = 'en' | 'zh'

export function setLocale(locale: Locale) {
  i18n.global.locale.value = locale
  localStorage.setItem('admin-locale', locale)
}
