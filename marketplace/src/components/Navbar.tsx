import { useState } from 'react';
import { NavLink, Link } from 'react-router-dom';
import { Menu, X, Github } from 'lucide-react';

export const Navbar: React.FC = () => {
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);

  const navItems = [
    { to: '/marketplace', label: 'Marketplace' },
    { to: '/docs', label: 'Docs' },
    { to: '/changelog', label: 'Changelog' },
  ];

  return (
    <header className="sticky top-0 z-40 w-full bg-slate-950/95 backdrop-blur-sm border-b border-slate-700">
      <div className="max-w-6xl mx-auto px-4 sm:px-6 lg:px-8 h-16 flex items-center justify-between">
        <Link
          to="/"
          className="font-sans font-extrabold text-2xl sm:text-[28px] tracking-tight text-white hover:text-emerald-400 transition-colors flex items-center gap-2.5 group"
        >
          <span className="w-2.5 h-2.5 rounded-full bg-emerald-400 inline-block animate-pulse shadow-[0_0_10px_rgba(52,211,153,0.8)]"></span>
          <span className="font-black tracking-tight">avm</span>
        </Link>

        <nav className="hidden md:flex items-center gap-7 text-[13px] text-slate-300">
          {navItems.map((item) => (
            <NavLink
              key={item.to}
              to={item.to}
              className={({ isActive }) =>
                isActive ? 'text-emerald-400 font-semibold' : 'text-slate-300 hover:text-slate-100 transition-colors'
              }
            >
              {item.label}
            </NavLink>
          ))}
          <a
            href="https://github.com/PrajaNova/avm"
            target="_blank"
            rel="noreferrer"
            className="flex items-center gap-2 text-slate-300 hover:text-white transition-colors px-2 py-1 rounded-lg hover:bg-slate-900"
            title="GitHub Repository"
            aria-label="GitHub Repository"
          >
            <Github className="w-4 h-4" />
            <span>GitHub</span>
          </a>
        </nav>

        <button
          onClick={() => setMobileMenuOpen(!mobileMenuOpen)}
          className="md:hidden p-2 -mr-2 text-slate-300 hover:text-white"
          aria-label="Toggle navigation menu"
        >
          {mobileMenuOpen ? <X className="w-5 h-5" /> : <Menu className="w-5 h-5" />}
        </button>
      </div>

      {mobileMenuOpen && (
        <div className="md:hidden border-t border-slate-700 px-6 py-4 flex flex-col gap-4 text-sm bg-slate-950">
          {navItems.map((item) => (
            <NavLink
              key={item.to}
              to={item.to}
              onClick={() => setMobileMenuOpen(false)}
              className={({ isActive }) => (isActive ? 'text-emerald-400 font-semibold' : 'text-slate-300')}
            >
              {item.label}
            </NavLink>
          ))}
          <a
            href="https://github.com/PrajaNova/avm"
            target="_blank"
            rel="noreferrer"
            className="flex items-center gap-2 text-slate-300 hover:text-white pt-2 border-t border-slate-800"
          >
            <Github className="w-4 h-4" />
            <span>GitHub</span>
          </a>
        </div>
      )}
    </header>
  );
};
