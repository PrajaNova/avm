import { useState } from 'react';
import { NavLink, Link } from 'react-router-dom';
import { Menu, X, Github, Moon, Sun, Terminal } from 'lucide-react';

export const Navbar = () => {
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);
  const [dark, setDark] = useState(() => document.documentElement.dataset.theme === 'dark');
  const navItems = [{ to: '/docs', label: 'Documentation' }, { to: '/marketplace', label: 'Plugins' }, { to: '/changelog', label: 'Changelog' }];
  const toggleTheme = () => {
    const theme = dark ? 'light' : 'dark';
    document.documentElement.dataset.theme = theme;
    setDark(!dark);
    try { localStorage.setItem('avm-theme', theme); } catch { /* Theme still works when storage is unavailable. */ }
  };
  return (
    <header className="chrome sticky top-0 z-40 border-b border-line bg-page/95 backdrop-blur-sm">
      <div className="max-w-7xl mx-auto px-5 sm:px-8 h-24 sm:h-[76px] flex items-center justify-between gap-6">
        <Link to="/" aria-label="AVM home" className="flex items-center gap-3 text-ink">
          <span className="bg-accent text-on-accent rounded-lg p-2.5 sm:p-1.5 sm:rounded-md"><Terminal className="w-7 h-7 sm:w-5 sm:h-5" aria-hidden="true" /></span>
          <span className="text-4xl sm:text-2xl font-bold tracking-tight">avm<span className="text-accent">.</span></span>
        </Link>
        <nav aria-label="Main navigation" className="hidden lg:flex items-center gap-7 text-sm">
          {navItems.map(item => <NavLink key={item.to} to={item.to} end={item.to !== '/docs'} className={({ isActive }) => isActive ? 'text-accent font-medium' : 'text-secondary hover:text-ink'}>{item.label}</NavLink>)}
        </nav>
        <div className="flex items-center gap-2">
          <a href="https://github.com/PrajaNova/avm" className="hidden sm:inline-flex items-center gap-2 px-3 min-h-11 text-sm text-secondary hover:text-ink"><Github className="w-4 h-4" /> GitHub</a>
          <button aria-label={dark ? 'Switch to light theme' : 'Switch to dark theme'} onClick={toggleTheme} className="p-3 rounded-md hover:bg-muted">{dark ? <Sun className="w-7 h-7 sm:w-5 sm:h-5" /> : <Moon className="w-7 h-7 sm:w-5 sm:h-5" />}</button>
          <button aria-label="Toggle navigation menu" aria-expanded={mobileMenuOpen} onClick={() => setMobileMenuOpen(!mobileMenuOpen)} className="lg:hidden p-3 rounded-md hover:bg-muted">{mobileMenuOpen ? <X className="w-7 h-7" /> : <Menu className="w-7 h-7" />}</button>
        </div>
      </div>
      {mobileMenuOpen && <nav aria-label="Mobile navigation" className="lg:hidden border-t border-line px-5 py-3 bg-surface flex flex-col">{navItems.map(item => <Link key={item.to} to={item.to} onClick={() => setMobileMenuOpen(false)} className="py-4 text-xl font-semibold text-ink hover:text-accent">{item.label}</Link>)}</nav>}
    </header>
  );
};
