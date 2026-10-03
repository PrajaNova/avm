import { Link } from 'react-router-dom';
import { Github, ExternalLink } from 'lucide-react';

export const Footer: React.FC = () => {
  return (
    <footer className="border-t border-slate-900 bg-slate-950 text-slate-400 text-xs font-sans mt-20">
      <div className="max-w-6xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
        <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-4 gap-8 mb-10">
          
          {/* Brand Col */}
          <div className="space-y-3">
            <Link to="/" className="flex items-center gap-2 text-white font-bold text-base">
              <span>avm</span>
            </Link>
            <p className="text-slate-400 text-xs leading-relaxed">
              Any Version Manager &mdash; a Rust-native tool for per-directory runtime versions, project aliases, and env vars, with verified plugins fetched on demand from the marketplace. macOS, Linux, and Windows.
            </p>
            <div className="text-[11px] text-slate-500 font-mono">
              Designed by PrajaNova &bull; MIT Licensed
            </div>
          </div>

          {/* Quick Links */}
          <div className="space-y-2">
            <div className="font-mono text-slate-200 font-semibold uppercase tracking-wider text-[11px]">
              Pages
            </div>
            <ul className="space-y-1.5 font-mono text-xs">
              <li>
                <Link to="/" className="hover:text-emerald-400 transition-colors">
                  Intro & Overview
                </Link>
              </li>
              <li>
                <Link to="/marketplace" className="hover:text-emerald-400 transition-colors">
                  Plugin Marketplace
                </Link>
              </li>
              <li>
                <Link to="/docs" className="hover:text-emerald-400 transition-colors">
                  Documentation & Architecture
                </Link>
              </li>
              <li>
                <Link to="/docs/manage/commands" className="hover:text-emerald-400 transition-colors">
                  CLI Command Reference
                </Link>
              </li>
              <li>
                <Link to="/changelog" className="hover:text-emerald-400 transition-colors">
                  Changelog
                </Link>
              </li>
            </ul>
          </div>

          {/* Ecosystem Repos */}
          <div className="space-y-2">
            <div className="font-mono text-slate-200 font-semibold uppercase tracking-wider text-[11px]">
              Repositories
            </div>
            <ul className="space-y-1.5 font-mono text-xs">
              <li>
                <a href="https://github.com/PrajaNova/avm" target="_blank" rel="noreferrer" className="hover:text-white flex items-center gap-1">
                  <span>PrajaNova/avm</span>
                  <ExternalLink className="w-3 h-3 text-slate-600" />
                </a>
              </li>
              <li>
                <a href="https://github.com/PrajaNova/avm/tree/main/marketplace" target="_blank" rel="noreferrer" className="hover:text-white flex items-center gap-1">
                  <span>PrajaNova/avm/marketplace</span>
                  <ExternalLink className="w-3 h-3 text-slate-600" />
                </a>
              </li>
              <li>
                <a href="https://github.com/PrajaNova/avm-plugin-node" target="_blank" rel="noreferrer" className="hover:text-white flex items-center gap-1">
                  <span>avm-plugin-node</span>
                  <ExternalLink className="w-3 h-3 text-slate-600" />
                </a>
              </li>
              <li>
                <a href="https://github.com/PrajaNova/avm-plugin-java" target="_blank" rel="noreferrer" className="hover:text-white flex items-center gap-1">
                  <span>avm-plugin-java</span>
                  <ExternalLink className="w-3 h-3 text-slate-600" />
                </a>
              </li>
              <li>
                <a href="https://github.com/PrajaNova/avm-plugin-android" target="_blank" rel="noreferrer" className="hover:text-white flex items-center gap-1">
                  <span>avm-plugin-android</span>
                  <ExternalLink className="w-3 h-3 text-slate-600" />
                </a>
              </li>
            </ul>
          </div>

          {/* Registry info */}
          <div className="space-y-2">
            <div className="font-mono text-slate-200 font-semibold uppercase tracking-wider text-[11px]">
              Registry Endpoint
            </div>
            <p className="text-[11px] text-slate-400">
              avm CLI fetches plugins on-demand directly from raw GitHub file:
            </p>
            <div className="p-2.5 rounded-lg bg-slate-900 border border-slate-800 font-mono text-[10px] text-emerald-400 break-all select-all">
              https://raw.githubusercontent.com/PrajaNova/avm/main/marketplace/registry.json
            </div>
          </div>

        </div>

        {/* Bottom banner */}
        <div className="border-t border-slate-900 pt-6 flex flex-col sm:flex-row items-center justify-between gap-4 text-[11px] text-slate-500 font-mono">
          <div>
            &copy; {new Date().getFullYear()} PrajaNova. All rights reserved.
          </div>
          <div className="flex items-center gap-4">
            <a href="https://github.com/PrajaNova/avm" target="_blank" rel="noreferrer" className="hover:text-slate-300 flex items-center gap-1">
              <Github className="w-3.5 h-3.5" />
              <span>GitHub</span>
            </a>
            <span className="text-slate-800">&bull;</span>
            <span>MIT License</span>
          </div>
        </div>
      </div>
    </footer>
  );
};
