import React, { useState } from 'react';
import {
  ChevronDown,
  ChevronRight,
  ExternalLink,
  BookOpen,
  X,
  PlayCircle
} from 'lucide-react';

import { ASDF_SIDEBAR_CONFIG } from '../../data/docsNav';
import type { SidebarItem, SidebarGroup } from '../../data/docsNav';

export { ASDF_SIDEBAR_CONFIG };
export type { SidebarItem, SidebarGroup };

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
            className="flex items-center justify-between px-3 py-1.5 rounded-lg text-xs font-sans text-secondary hover:text-ink hover:bg-surface/60 transition-colors"
          >
            <span className="truncate">{item.text}</span>
            <ExternalLink className="w-3 h-3 text-secondary shrink-0 ml-1.5" />
          </a>
        </li>
      );
    }

    return (
      <li key={item.id}>
        <button
          aria-current={isActive ? 'page' : undefined}
          onClick={() => {
            onSelectSection(item.link);
            if (onCloseMobile) onCloseMobile();
          }}
          className={`w-full flex items-center justify-between text-left px-3 py-1.5 rounded-lg text-xs font-sans transition-all group ${
            isActive
              ? 'bg-accent/10 text-accent font-semibold border-l-2 border-accent shadow-sm'
              : 'text-secondary hover:text-ink hover:bg-surface/50'
          }`}
        >
          <span className="truncate">{item.text}</span>
          {item.hasVideo && (
            <span className="flex items-center gap-1 text-xs font-mono px-1.5 py-0.2 rounded bg-accent/15 text-accent border border-accent/25 shrink-0 ml-1.5 font-medium">
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
          className="fixed inset-0 z-40 bg-page/80 backdrop-blur-sm lg:hidden transition-opacity"
        />
      )}

      {/* Sidebar Navigation */}
      <aside
        className={`fixed left-0 top-0 bottom-0 z-40 w-72 bg-page border-r border-line lg:bg-transparent lg:static lg:w-64 lg:z-auto transition-transform duration-200 ease-in-out flex flex-col justify-between shrink-0 ${
          isOpenMobile ? 'visible translate-x-0' : 'invisible lg:visible -translate-x-full lg:translate-x-0'
        }`}
      >
        <div className="flex-1 overflow-y-auto px-4 py-6">
          
          {/* Mobile Header with close button */}
          <div className="flex items-center justify-between pb-4 mb-4 border-b border-line lg:hidden">
            <div className="flex items-center gap-2">
              <BookOpen className="w-4 h-4 text-accent" />
              <span className="font-sans text-sm font-bold text-ink">Documentation</span>
            </div>
            {onCloseMobile && (
              <button
                onClick={onCloseMobile}
                className="p-1 rounded-lg text-secondary hover:text-ink hover:bg-muted transition-colors"
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
                <div key={group.text} className="border-b border-line/40 pb-4 last:border-b-0">
                  
                  {/* Group Title with Collapse Toggle */}
                  <button
                    aria-expanded={!isCollapsed}
                    onClick={() => toggleGroup(group.text)}
                    className="w-full flex items-center justify-between text-left px-2 py-1 text-xs font-bold uppercase tracking-wider text-secondary hover:text-ink transition-colors"
                  >
                    <span>{group.text}</span>
                    <span className="text-secondary group-hover:text-secondary">
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
        <div className="p-4 border-t border-line/80 bg-page/40 text-xs">
          <a
            href="https://github.com/PrajaNova/avm"
            target="_blank"
            rel="noreferrer"
            className="flex items-center justify-between text-secondary hover:text-ink transition-colors font-mono text-xs"
          >
            <span>GitHub Repository</span>
            <ExternalLink className="w-3.5 h-3.5 text-secondary" />
          </a>
        </div>
      </aside>
    </>
  );
};
