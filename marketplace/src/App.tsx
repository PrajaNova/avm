import { useState } from 'react';
import { HashRouter, Routes, Route, Navigate, useParams } from 'react-router-dom';
import { Navbar } from './components/Navbar';
import { Footer } from './components/Footer';
import { Toast } from './components/Toast';
import { ScrollToTop } from './components/ScrollToTop';
import { Marketplace } from './components/Marketplace';

import { HomePage } from './pages/HomePage';
import { DocsPage } from './pages/DocsPage';
import { PluginPage } from './pages/PluginPage';
import { ChangelogPage } from './pages/ChangelogPage';

const PluginRedirect = () => <Navigate to={`/marketplace/${useParams().name}`} replace />;

export function App() {
  const [toastMessage, setToastMessage] = useState<string | null>(null);

  const copy = (text: string) => {
    navigator.clipboard.writeText(text);
    setToastMessage(`Copied: ${text}`);
    setTimeout(() => setToastMessage(null), 2500);
  };

  return (
    <HashRouter>
      <div className="min-h-screen bg-page text-ink flex flex-col antialiased selection:bg-accent/30 selection:text-accent">
        <ScrollToTop />
        <Toast message={toastMessage} />

        {/* Global sticky navigation */}
        <Navbar />

        {/* Routed pages */}
        <main className="flex-1">
          <Routes>
            <Route path="/" element={<HomePage onCopy={copy} />} />
            <Route path="/marketplace" element={<div className="py-6"><Marketplace onCopy={copy} /></div>} />
            <Route path="/marketplace/:name" element={<PluginPage onCopy={copy} />} />
            <Route path="/docs/plugins/:name" element={<PluginRedirect />} />
            <Route path="/docs/*" element={<DocsPage onCopy={copy} />} />
            <Route path="/changelog" element={<ChangelogPage />} />
            <Route path="/commands" element={<Navigate to="/docs/manage/commands" replace />} />
            <Route path="*" element={<Navigate to="/" replace />} />
          </Routes>
        </main>

        {/* Global footer */}
        <Footer />
      </div>
    </HashRouter>
  );
}

export default App;
