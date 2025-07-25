import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import HttpBackend from 'i18next-http-backend';
import LanguageDetector from 'i18next-browser-languagedetector';

i18n
  .use(HttpBackend)
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    fallbackLng: 'en',
    debug: true,
    interpolation: {
      escapeValue: false, // React already safes from xss
      prefix: '%{',      // Set prefix to match rust-i18n
      suffix: '}'        // Set suffix to match rust-i18n
    },
    backend: {
      loadPath: '/locales/%{lng}.json',
    },
  });

export default i18n;