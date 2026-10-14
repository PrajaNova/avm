import { Link } from 'react-router';
import { Github, ExternalLink } from 'lucide-react';

export const Footer: React.FC = () => {
  return (
    <footer className="chrome border-t border-line bg-page text-secondary text-xs font-sans mt-20">
      <div className="max-w-6xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
        <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-4 gap-8 mb-10">
          
          {/* Brand Col */}
          <div className="space-y-3">
            <Link to="/" className="flex items-center gap-2 text-ink font-bold text-base">
              <span>avm</span>
            </Link>
            <p className="text-secondary text-xs leading-relaxed">
              Any Version Manager &mdash; a Rust-native tool for per-directory runtime versions, project aliases, and env vars, with verified plugins fetched on demand from the marketplace. macOS, Linux, and Windows.
            </p>
            <div className="text-xs text-secondary font-mono">
              Designed by PrajaNova &bull; MIT Licensed
            </div>
          </div>

          {/* Quick Links */}
          <div className="space-y-2">
            <div className="font-mono text-ink font-semibold uppercase tracking-wider text-xs">
              Pages
            </div>
            <ul className="space-y-1.5 font-mono text-xs">
              <li>
                <Link to="/" className="hover:text-accent transition-colors">
                  Intro & Overview
                </Link>
              </li>
              <li>
                <Link to="/marketplace" className="hover:text-accent transition-colors">
                  Plugin Marketplace
                </Link>
              </li>
              <li>
                <Link to="/docs" className="hover:text-accent transition-colors">
                  Documentation & Architecture
                </Link>
              </li>
              <li>
                <Link to="/docs/manage/commands" className="hover:text-accent transition-colors">
                  CLI Command Reference
                </Link>
              </li>
              <li>
                <Link to="/changelog" className="hover:text-accent transition-colors">
                  Changelog
                </Link>
              </li>
            </ul>
          </div>

          {/* Ecosystem Repos */}
          <div className="space-y-2">
            <div className="font-mono text-ink font-semibold uppercase tracking-wider text-xs">
              Repository
            </div>
            <ul className="space-y-1.5 font-mono text-xs">
              <li>
                <a href="https://github.com/PrajaNova/avm" target="_blank" rel="noreferrer" className="hover:text-ink flex items-center gap-1">
                  <span>PrajaNova/avm</span>
                  <ExternalLink className="w-3 h-3 text-secondary" />
                </a>
              </li>
            </ul>
          </div>

          {/* Registry info */}
          <div className="space-y-2">
            <div className="font-mono text-ink font-semibold uppercase tracking-wider text-xs">
              Registry Endpoint
            </div>
            <p className="text-xs text-secondary">
              avm CLI fetches plugins on-demand directly from raw GitHub file:
            </p>
            <div className="p-2.5 rounded-lg bg-surface border border-line font-mono text-xs text-accent break-all select-all">
              https://raw.githubusercontent.com/PrajaNova/avm/main/marketplace/registry.json
            </div>
          </div>

        </div>

        {/* Bottom banner */}
        <div className="border-t border-line pt-6 flex flex-col sm:flex-row items-center justify-between gap-4 text-xs text-secondary font-mono">
          <div>
            &copy; {new Date().getFullYear()} PrajaNova. All rights reserved.
          </div>
          <div className="flex items-center gap-4">
            <a href="https://github.com/PrajaNova/avm" target="_blank" rel="noreferrer" className="hover:text-secondary flex items-center gap-1">
              <Github className="w-3.5 h-3.5" />
              <span>GitHub</span>
            </a>
            <span className="text-secondary">&bull;</span>
            <span>MIT License</span>
          </div>
        </div>
      </div>
    </footer>
  );
};
