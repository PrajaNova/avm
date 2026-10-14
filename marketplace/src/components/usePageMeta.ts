import { useEffect } from 'react';

// Keeps the tab title (and robots hint) in step with client-side navigation.
// Crawlers get the same values from the prerendered HTML written by scripts/seo.mjs.
export const usePageMeta = (title: string, noindex = false) => {
  useEffect(() => {
    document.title = title;
    const tag = document.querySelector('meta[name="robots"]') ?? document.head.appendChild(Object.assign(document.createElement('meta'), { name: 'robots' }));
    tag.setAttribute('content', noindex ? 'noindex' : 'index, follow');
  }, [title, noindex]);
};
