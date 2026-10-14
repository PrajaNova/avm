// Post-build SEO step: writes one HTML file per route (own title, description, canonical, social tags),
// plus sitemap.xml. The app is a client-rendered SPA, so this is what non-JS crawlers and link previews see.
// ponytail: head tags only; the body is still rendered client-side. Move to a real prerenderer if rich snippets need body text.
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import ts from 'typescript';

const SITE = 'https://getavm.dev';
const HOME_TITLE = 'AVM: Any Version Manager for Node.js, Java and Android';
const HOME_DESC = 'AVM manages Node.js, Java and Android SDK versions per project, with aliases, environment variables and lockfiles. Open source for macOS, Linux and Windows.';
const dist = (path) => new URL(`../dist/${path}`, import.meta.url);
const src = (path) => new URL(`../${path}`, import.meta.url);

async function data(path) {
  const { outputText } = ts.transpileModule(await readFile(src(path), 'utf8'), { compilerOptions: { module: ts.ModuleKind.ESNext } });
  return import(`data:text/javascript;base64,${Buffer.from(outputText).toString('base64')}`);
}
const esc = (text) => text.replace(/&/g, '&amp;').replace(/"/g, '&quot;').replace(/</g, '&lt;');
const clip = (text, max = 155) => (text.length <= max ? text : `${text.slice(0, text.lastIndexOf(' ', max - 1))}…`);

const { FEATURE_GUIDES } = await data('src/data/featureGuides.ts');
const registry = JSON.parse(await readFile(src('registry.json'), 'utf8'));
const sidebar = await readFile(src('src/components/docs/DocsSidebar.tsx'), 'utf8');
const docs = [...sidebar.matchAll(/id: '([^']+)',\s*text: '((?:[^'\\]|\\.)*)',\s*link: '(\/docs\/[^']+)'/g)]
  .map(([, id, text, path]) => ({ id, text: text.replace(/\\'/g, "'"), path }));

const routes = [
  { path: '/', title: HOME_TITLE, description: HOME_DESC, home: true },
  { path: '/marketplace', title: 'AVM plugins: Node.js, Java and Android SDK', description: 'Official AVM plugins for Node.js, Java (Temurin OpenJDK) and the Android SDK. Each lists supported platforms, lockfile support and its install command.' },
  { path: '/changelog', title: 'AVM changelog and release notes', description: 'What changed in each AVM release: new commands, plugin updates and fixes.' },
  ...docs.map(({ id, text, path }) => {
    const guide = FEATURE_GUIDES[id];
    return { path, title: `${guide?.title ?? text} | AVM docs`, description: clip(guide?.intro ?? `${text}: AVM documentation for managing Node.js, Java and Android SDK versions per project.`) };
  }),
  ...registry.plugins.map((p) => ({ path: `/marketplace/${p.name}`, title: `${p.section_label} plugin for AVM`, description: clip(`${p.description}. Install with avm plugin add ${p.name}.`) })),
  // /docs renders the first docs page, so it points search engines there instead of competing with it.
  { path: '/docs', title: `${docs[0].text} | AVM docs`, description: HOME_DESC, canonical: docs[0].path, skipSitemap: true },
];

const jsonLd = (route) => !route.home ? '' : `\n    <script type="application/ld+json">${JSON.stringify({
  '@context': 'https://schema.org',
  '@graph': [
    { '@type': 'WebSite', name: 'AVM', alternateName: 'Any Version Manager', url: `${SITE}/` },
    { '@type': 'SoftwareApplication', name: 'AVM', alternateName: 'Any Version Manager', description: HOME_DESC, url: `${SITE}/`, applicationCategory: 'DeveloperApplication', operatingSystem: 'macOS, Linux, Windows', license: 'https://opensource.org/licenses/MIT', codeRepository: 'https://github.com/PrajaNova/avm', offers: { '@type': 'Offer', price: '0', priceCurrency: 'USD' } },
  ],
})}</script>`;

const template = await readFile(dist('index.html'), 'utf8');
for (const route of routes) {
  const url = `${SITE}${route.canonical ?? route.path}`;
  const head = [
    `<link rel="canonical" href="${url}" />`,
    `<meta property="og:type" content="website" />`,
    `<meta property="og:site_name" content="AVM" />`,
    `<meta property="og:title" content="${esc(route.title)}" />`,
    `<meta property="og:description" content="${esc(route.description)}" />`,
    `<meta property="og:url" content="${url}" />`,
    `<meta property="og:image" content="${SITE}/og.png" />`,
    `<meta name="twitter:card" content="summary_large_image" />`,
  ].join('\n    ') + jsonLd(route);
  const html = template
    .replace(/<title>.*?<\/title>/, `<title>${esc(route.title)}</title>`)
    .replace(/<meta name="description"[^>]*>/, `<meta name="description" content="${esc(route.description)}" />`)
    .replace('</head>', `    ${head}\n  </head>`)
    .replace('<div id="root"></div>', `<div id="root"></div>\n    <noscript><h1>${esc(route.title)}</h1><p>${esc(route.description)}</p></noscript>`);
  const file = route.path === '/' ? 'index.html' : `${route.path.slice(1)}/index.html`;
  await mkdir(new URL('./', dist(file)), { recursive: true });
  await writeFile(dist(file), html);
}

const today = new Date().toISOString().slice(0, 10);
const urls = routes.filter((r) => !r.skipSitemap).map((r) => `  <url><loc>${SITE}${r.path}</loc><lastmod>${today}</lastmod></url>`);
await writeFile(dist('sitemap.xml'), `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${urls.join('\n')}\n</urlset>\n`);
console.log(`seo: ${routes.length} pages, ${urls.length} in sitemap`);
