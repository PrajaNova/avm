import { useState } from 'react';
import { BrowserRouter, Routes, Route, Navigate, useParams } from 'react-router-dom';
import { Navbar } from './components/Navbar';
import { Footer } from './components/Footer';
import { Toast } from './components/Toast';
import { ScrollToTop } from './components/ScrollToTop';
import { Marketplace } from './components/Marketplace';

import { HomePage } from './pages/HomePage';
import { DocsPage } from './pages/DocsPage';
import { usePageMeta } from './components/usePageMeta';
import { PluginPage } from './pages/PluginPage';
import { NotFoundPage } from './pages/NotFoundPage';
import { ChangelogPage } from './pages/ChangelogPage';

// Titles here must match scripts/seo.mjs, which writes the same values into the prerendered HTML.
const HOME_TITLE = 'AVM: Any Version Manager for Node.js, Java and Android';

const Titled = ({ title, children }: { title: string; children: React.ReactNode }) => {
  usePageMeta(title);
  return <>{children}</>;
};

const PluginRedirect = () => <Navigate to={`/marketplace/${useParams().name}`} replace />;

export function App() {
  const [toastMessage, setToastMessage] = useState<string | null>(null);

  const copy = (text: string) => {
    navigator.clipboard.writeText(text);
    setToastMessage(`Copied: ${text}`);
    setTimeout(() => setToastMessage(null), 2500);
  };

  return (
    <BrowserRouter>
      <div className="min-h-screen bg-page text-ink flex flex-col antialiased selection:bg-accent/30 selection:text-accent">
        <ScrollToTop />
        <Toast message={toastMessage} />

        {/* Global sticky navigation */}
        <Navbar />

        {/* Routed pages */}
        <main className="flex-1">
          <Routes>
            <Route path="/" element={<Titled title={HOME_TITLE}><HomePage onCopy={copy} /></Titled>} />
            <Route path="/marketplace" element={<Titled title="AVM plugins: Node.js, Java and Android SDK"><div className="py-6"><Marketplace onCopy={copy} /></div></Titled>} />
            <Route path="/marketplace/:name" element={<PluginPage onCopy={copy} />} />
            <Route path="/docs/plugins/:name" element={<PluginRedirect />} />
            <Route path="/docs/*" element={<DocsPage onCopy={copy} />} />
            <Route path="/changelog" element={<Titled title="AVM changelog and release notes"><ChangelogPage /></Titled>} />
            <Route path="/commands" element={<Navigate to="/docs/manage/commands" replace />} />
            <Route path="*" element={<NotFoundPage />} />
          </Routes>
        </main>

        {/* Global footer */}
        <Footer />
      </div>
    </BrowserRouter>
  );
}

export default App;
