import { Links, Meta, Outlet, Scripts, ScrollRestoration } from 'react-router';
import { Analytics } from '@vercel/analytics/react';
import { Navbar } from './components/Navbar';
import { Footer } from './components/Footer';
import { CopyProvider } from './components/CopyContext';
import { NotFoundPage } from './pages/NotFoundPage';
import { isRouteErrorResponse, useRouteError } from 'react-router';
import '@fontsource/ibm-plex-sans/latin-400.css';
import '@fontsource/ibm-plex-sans/latin-500.css';
import '@fontsource/ibm-plex-sans/latin-600.css';
import '@fontsource/ibm-plex-sans/latin-700.css';
import '@fontsource/ibm-plex-mono/latin-400.css';
import '@fontsource/ibm-plex-mono/latin-500.css';
import '@fontsource/ibm-plex-mono/latin-600.css';
import './index.css';

export const links = () => [
  { rel: 'icon', type: 'image/svg+xml', href: '/favicon.svg' },
];

// Runs before first paint: restores old #/path links and the saved theme.
const BOOT = `if (location.hash.indexOf('#/') === 0) location.replace(location.hash.slice(1));
try { document.documentElement.dataset.theme = localStorage.getItem('avm-theme') === 'dark' ? 'dark' : 'light'; } catch { document.documentElement.dataset.theme = 'light'; }`;

export function Layout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" suppressHydrationWarning>
      <head>
        <meta charSet="UTF-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <meta name="theme-color" content="#FAFAF9" />
        <script dangerouslySetInnerHTML={{ __html: BOOT }} />
        <Meta />
        <Links />
      </head>
      <body className="bg-page text-ink min-h-screen selection:bg-accent/30 selection:text-accent">
        <CopyProvider>
          <div className="min-h-screen bg-page text-ink flex flex-col antialiased">
            <Navbar />
            <main className="flex-1">{children}</main>
            <Footer />
          </div>
        </CopyProvider>
        <Analytics />
        <ScrollRestoration />
        <Scripts />
      </body>
    </html>
  );
}

export default function Root() {
  return <Outlet />;
}

export function ErrorBoundary() {
  const error = useRouteError();
  if (isRouteErrorResponse(error) && error.status !== 404) {
    return <div className="max-w-2xl mx-auto px-5 py-24"><h1 className="text-4xl font-medium mb-4">{error.status}</h1><p className="text-secondary">{error.statusText}</p></div>;
  }
  return <NotFoundPage />;
}
