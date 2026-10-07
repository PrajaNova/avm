import { useState } from 'react';
import { HashRouter, Routes, Route, Navigate } from 'react-router-dom';
import { Navbar } from './components/Navbar';
import { Footer } from './components/Footer';
import { Toast } from './components/Toast';
import { ScrollToTop } from './components/ScrollToTop';
import { Marketplace } from './components/Marketplace';

import { HomePage } from './pages/HomePage';
import { DocsPage } from './pages/DocsPage';
import { ChangelogPage } from './pages/ChangelogPage';

export function App() {
  const [toastMessage, setToastMessage] = useState<string | null>(null);

  const copy = (text: string) => {
    navigator.clipboard.writeText(text);
    setToastMessage(`Copied: ${text}`);
    setTimeout(() => setToastMessage(null), 2500);
  };

  return (
    <HashRouter>
      <div className="min-h-screen bg-slate-950 text-slate-100 flex flex-col antialiased selection:bg-emerald-500/30 selection:text-emerald-300">
        <ScrollToTop />
        <Toast message={toastMessage} />

        {/* Global sticky navigation */}
        <Navbar />

        {/* Routed pages */}
        <main className="flex-1">
          <Routes>
            <Route path="/" element={<HomePage onCopy={copy} />} />
            <Route path="/marketplace" element={<div className="py-6"><Marketplace onCopy={copy} /></div>} />
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
