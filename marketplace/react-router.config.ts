import type { Config } from '@react-router/dev/config';
import { SEO_PAGES } from './src/data/seo';

// Static site: every page in SEO_PAGES is rendered to HTML at build time; other URLs use the SPA fallback (served as 404).
export default {
  appDirectory: 'src',
  ssr: false,
  prerender: SEO_PAGES.map((page) => page.path),
} satisfies Config;
