// After `react-router build`: sitemap.xml from the prerendered pages, registry.json,
// and the SPA fallback as 404.html so Vercel answers unknown URLs with a 404 status.
import { execFileSync } from 'node:child_process';
import { copyFile, readdir, writeFile } from 'node:fs/promises';

const SITE = 'https://getavm.dev';
const root = new URL('../build/client/', import.meta.url);
const files = await readdir(root, { recursive: true });
// /docs duplicates its first page and says so with a canonical link, so it stays out of the sitemap.
const paths = files.filter((f) => f.endsWith('index.html') && f !== 'docs/index.html').map((f) => `/${f.replace(/index\.html$/, '')}`.replace(/(.)\/$/, '$1')).sort();

// Date of the last commit touching the site, so lastmod only moves when content does; falls back to today without git history.
const lastmod = (() => {
  try { return execFileSync('git', ['log', '-1', '--format=%cs', '--', 'src', '../registry.json', 'public'], { cwd: new URL('..', import.meta.url), encoding: 'utf8' }).trim() || undefined; } catch { return undefined; }
})() ?? new Date().toISOString().slice(0, 10);
const urls = paths.map((path) => `  <url><loc>${SITE}${path}</loc><lastmod>${lastmod}</lastmod></url>`);
await writeFile(new URL('sitemap.xml', root), `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${urls.join('\n')}\n</urlset>\n`);
await copyFile(new URL('../../registry.json', import.meta.url), new URL('registry.json', root));
await copyFile(new URL('__spa-fallback.html', root), new URL('404.html', root));
console.log(`postbuild: ${urls.length} sitemap URLs`);
