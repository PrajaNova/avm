import registry from '../../registry.json';
import { ASDF_SIDEBAR_CONFIG } from './docsNav';
import { FEATURE_GUIDES } from './featureGuides';
import { SEO_GUIDES } from './seoGuides';

export const SITE = 'https://getavm.dev';
export const HOME_TITLE = 'AVM: Any Version Manager for Node.js, Java and Android';
export const HOME_DESC = 'AVM manages Node.js, Java and Android SDK versions per project, with aliases, environment variables and lockfiles. Open source for macOS, Linux and Windows.';

export interface PageSeo {
  path: string;
  title: string;
  description: string;
  canonical?: string;
  inSitemap: boolean;
}

const clip = (text: string, max = 155) => (text.length <= max ? text : `${text.slice(0, text.lastIndexOf(' ', max - 1))}…`);

const docs = ASDF_SIDEBAR_CONFIG.flatMap((g) => g.items.filter((i) => !i.external));

export const SEO_PAGES: PageSeo[] = [
  { path: '/', title: HOME_TITLE, description: HOME_DESC, inSitemap: true },
  { path: '/marketplace', title: 'AVM plugins: Node.js, Java and Android SDK', description: 'Official AVM plugins for Node.js, Java (Temurin OpenJDK) and the Android SDK. Each lists supported platforms, lockfile support and its install command.', inSitemap: true },
  { path: '/changelog', title: 'AVM changelog and release notes', description: 'What changed in each AVM release: new commands, plugin updates and fixes.', inSitemap: true },
  ...docs.map(({ id, text, link }) => {
    const guide = FEATURE_GUIDES[id];
    return { path: link, title: `${guide?.title ?? text} | AVM docs`, description: clip(guide?.intro ?? `${text}: AVM documentation for managing Node.js, Java and Android SDK versions per project.`), inSitemap: true };
  }),
  ...registry.plugins.map((p) => ({ path: `/marketplace/${p.name}`, title: `${p.section_label} plugin for AVM`, description: clip(`${p.description}. Install with avm plugin add ${p.name}.`), inSitemap: true })),
  ...SEO_GUIDES.map((g) => ({ path: `/guides/${g.slug}`, title: g.title, description: g.description, inSitemap: true })),
  // /docs renders the first docs page, so it points search engines there instead of competing with it.
  { path: '/docs', title: `${docs[0].text} | AVM docs`, description: HOME_DESC, canonical: docs[0].link, inSitemap: false },
];

export const seoFor = (path: string): PageSeo | undefined => SEO_PAGES.find((p) => p.path === path.replace(/(.)\/$/, '$1'));

const NOT_FOUND = { title: 'Page not found | AVM', description: 'This page does not exist.' };

// Descriptors for a route's `meta` export: title, description, canonical, social tags and (home) JSON-LD.
export const pageMeta = (path: string) => {
  const page = seoFor(path);
  if (!page) return [{ title: NOT_FOUND.title }, { name: 'robots', content: 'noindex' }];
  const url = `${SITE}${page.canonical ?? page.path}`;
  const tags: Record<string, string>[] = [
    { title: page.title },
    { name: 'description', content: page.description },
    { tagName: 'link', rel: 'canonical', href: url },
    { property: 'og:type', content: 'website' },
    { property: 'og:site_name', content: 'AVM' },
    { property: 'og:title', content: page.title },
    { property: 'og:description', content: page.description },
    { property: 'og:url', content: url },
    { property: 'og:image', content: `${SITE}/og.png` },
    { name: 'twitter:card', content: 'summary_large_image' },
  ];
  if (page.path !== '/') return tags;
  return [...tags, { 'script:ld+json': {
    '@context': 'https://schema.org',
    '@graph': [
      { '@type': 'WebSite', name: 'AVM', alternateName: 'Any Version Manager', url: `${SITE}/` },
      { '@type': 'SoftwareApplication', name: 'AVM', alternateName: 'Any Version Manager', description: HOME_DESC, url: `${SITE}/`, applicationCategory: 'DeveloperApplication', operatingSystem: 'macOS, Linux, Windows', license: 'https://opensource.org/licenses/MIT', codeRepository: 'https://github.com/PrajaNova/avm', offers: { '@type': 'Offer', price: '0', priceCurrency: 'USD' } },
    ],
  } } as unknown as Record<string, string>];
};
