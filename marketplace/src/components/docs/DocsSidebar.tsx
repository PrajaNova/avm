import React, { useState } from 'react';
import {
  ChevronDown,
  ChevronRight,
  ExternalLink,
  BookOpen,
  X,
  PlayCircle
} from 'lucide-react';

export interface SidebarItem {
  id: string;
  text: string;
  link: string;
  external?: boolean;
  hasVideo?: boolean;
  badge?: string;
}

export interface SidebarGroup {
  text: string;
  items: SidebarItem[];
}

export const ASDF_SIDEBAR_CONFIG: SidebarGroup[] = [
  {
    text: 'Guide',
    items: [
      { id: 'introduction', text: 'What is avm?', link: '/docs/guide/introduction' },
      { id: 'getting-started', text: 'Getting Started', link: '/docs/guide/getting-started' },
      { id: 'shell-setup', text: 'Shell Setup & Shims', link: '/docs/guide/shell-setup' },
    ],
  },
  {
    text: 'Usage',
    items: [
      { id: 'usage-core', text: 'Core (CLI & Precedence)', link: '/docs/manage/core' },
      { id: 'usage-plugins', text: 'Plugins (Add, List, Update)', link: '/docs/manage/plugins' },
      { id: 'usage-versions', text: 'Versions (Browse, Install, Pin)', link: '/docs/manage/versions' },
      { id: 'usage-aliases', text: 'Aliases & Scripts Discovery', link: '/docs/manage/aliases' },
      { id: 'version-files', text: 'Version Files (.nvmrc, .tool-versions)', link: '/docs/manage/version-files', badge: 'New' },
      { id: 'security', text: 'Security & Trust', link: '/docs/manage/security', badge: 'New' },
    ],
  },
  {
    text: 'Reference',
    items: [
      { id: 'configuration', text: 'Configuration (.avm.json)', link: '/docs/manage/configuration' },
      { id: 'commands', text: 'All Commands', link: '/docs/manage/commands' },
      { id: 'dependencies', text: 'Dependencies & Host Env', link: '/docs/manage/dependencies' },
      { id: 'architecture', text: 'Architecture & Wire Protocol', link: '/docs/manage/architecture' },
      { id: 'comparison', text: 'Comparison Matrix', link: '/docs/manage/comparison' },
    ],
  },
  {
    text: 'Plugins',
    items: [
      { id: 'plugin-node', text: 'Node.js', link: '/docs/plugins/node', hasVideo: true, badge: 'Walkthrough' },
      { id: 'plugin-java', text: 'Java (Temurin OpenJDK)', link: '/docs/plugins/java', hasVideo: true, badge: 'Walkthrough' },
      { id: 'plugin-android', text: 'Android SDK & AVD', link: '/docs/plugins/android', hasVideo: true, badge: 'Walkthrough' },
    ],
  },
  {
    text: 'Questions',
    items: [
      { id: 'faq', text: 'FAQ', link: '/docs/more/faq' },
      { id: 'troubleshooting', text: 'Troubleshooting & Diagnostics', link: '/docs/more/troubleshooting' },
      {
        id: 'github-issues',
        text: 'GitHub Issues',
        link: 'https://github.com/PrajaNova/avm/issues',
        external: true,
      },
    ],
  },
  {
    text: 'Contribute',
    items: [
      { id: 'contribute-core', text: 'Core avm', link: '/docs/contribute/core' },
      { id: 'contribute-docs', text: 'Documentation', link: '/docs/contribute/documentation' },
    ],
  },
];

interface DocsSidebarProps {
  activeSection: string;
  onSelectSection: (link: string) => void;
  isOpenMobile?: boolean;
  onCloseMobile?: () => void;
}

export const DocsSidebar: React.FC<DocsSidebarProps> = ({
  activeSection,
  onSelectSection,
  isOpenMobile,
  onCloseMobile,
}) => {
  // Track collapsed status for each group
  const [collapsedGroups, setCollapsedGroups] = useState<Record<string, boolean>>({
    Contribute: true, // collapsed by default like asdf
  });

  const toggleGroup = (groupTitle: string) => {
    setCollapsedGroups((prev) => ({
      ...prev,
      [groupTitle]: !prev[groupTitle],
    }));
  };

  const renderItem = (item: SidebarItem) => {
    const isActive = activeSection === item.id;

    if (item.external) {
      return (
        <li key={item.id}>
          <a
            href={item.link}
            target="_blank"
            rel="noreferrer"
            className="flex items-center justify-between px-3 py-1.5 rounded-lg text-xs font-sans text-slate-400 hover:text-slate-200 hover:bg-slate-900/60 transition-colors"
          >
            <span className="truncate">{item.text}</span>
            <ExternalLink className="w-3 h-3 text-slate-500 shrink-0 ml-1.5" />
          </a>
        </li>
      );
    }

    return (
      <li key={item.id}>
        <button
          onClick={() => {
            onSelectSection(item.link);
            if (onCloseMobile) onCloseMobile();
          }}
          className={`w-full flex items-center justify-between text-left px-3 py-1.5 rounded-lg text-xs font-sans transition-all group ${
            isActive
              ? 'bg-emerald-500/10 text-emerald-300 font-semibold border-l-2 border-emerald-400 shadow-sm'
              : 'text-slate-400 hover:text-slate-200 hover:bg-slate-900/50'
          }`}
        >
          <span className="truncate">{item.text}</span>
          {item.hasVideo && (
            <span className="flex items-center gap-1 text-[10px] font-mono px-1.5 py-0.2 rounded bg-emerald-500/15 text-emerald-400 border border-emerald-500/25 shrink-0 ml-1.5 font-medium">
              <PlayCircle className="w-2.5 h-2.5" />
              <span>{item.badge || 'Video'}</span>
            </span>
          )}
        </button>
      </li>
    );
  };

  return (
    <>
      {/* Mobile Backdrop */}
      {isOpenMobile && (
        <div
          onClick={onCloseMobile}
          className="fixed inset-0 z-40 bg-slate-950/80 backdrop-blur-sm lg:hidden transition-opacity"
        />
      )}

      {/* Sidebar Navigation */}
      <aside
        className={`fixed top-0 bottom-0 z-40 w-72 bg-slate-950 border-r border-slate-800 lg:bg-transparent lg:static lg:w-64 lg:z-auto transition-transform duration-200 ease-in-out flex flex-col justify-between shrink-0 ${
          isOpenMobile ? 'translate-x-0' : '-translate-x-full lg:translate-x-0'
        }`}
      >
        <div className="flex-1 overflow-y-auto px-4 py-6">
          
          {/* Mobile Header with close button */}
          <div className="flex items-center justify-between pb-4 mb-4 border-b border-slate-800 lg:hidden">
            <div className="flex items-center gap-2">
              <BookOpen className="w-4 h-4 text-emerald-400" />
              <span className="font-sans text-sm font-bold text-white">Documentation</span>
            </div>
            {onCloseMobile && (
              <button
                onClick={onCloseMobile}
                className="p-1 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800 transition-colors"
                title="Close sidebar"
              >
                <X className="w-5 h-5" />
              </button>
            )}
          </div>

          {/* asdf-style section-by-section navigation */}
          <div className="space-y-6">
            {ASDF_SIDEBAR_CONFIG.map((group) => {
              const isCollapsed = !!collapsedGroups[group.text];
              return (
                <div key={group.text} className="border-b border-slate-800/40 pb-4 last:border-b-0">
                  
                  {/* Group Title with Collapse Toggle */}
                  <button
                    onClick={() => toggleGroup(group.text)}
                    className="w-full flex items-center justify-between text-left px-2 py-1 text-xs font-bold uppercase tracking-wider text-slate-300 hover:text-white transition-colors"
                  >
                    <span>{group.text}</span>
                    <span className="text-slate-500 group-hover:text-slate-400">
                      {isCollapsed ? (
                        <ChevronRight className="w-3.5 h-3.5" />
                      ) : (
                        <ChevronDown className="w-3.5 h-3.5" />
                      )}
                    </span>
                  </button>

                  {/* Group Items */}
                  {!isCollapsed && (
                    <ul className="mt-1 space-y-0.5 pl-1">
                      {group.items.map(renderItem)}
                    </ul>
                  )}

                </div>
              );
            })}
          </div>

        </div>

        {/* Sidebar Footer Link */}
        <div className="p-4 border-t border-slate-800/80 bg-slate-950/40 text-xs">
          <a
            href="https://github.com/PrajaNova/avm"
            target="_blank"
            rel="noreferrer"
            className="flex items-center justify-between text-slate-400 hover:text-white transition-colors font-mono text-[11px]"
          >
            <span>GitHub Repository</span>
            <ExternalLink className="w-3.5 h-3.5 text-slate-500" />
          </a>
        </div>
      </aside>
    </>
  );
};
