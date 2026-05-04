//! AI Studio Navigation Component
//!
//! Provides breadcrumb navigation and quick links to specialized AI tools.
//! This component ties together AI Studio (front door) with deep-linked specialized pages.
//! Uses plain DOM navigation instead of React Router to work in non-Router contexts.

import React from 'react';
import { ChevronRight, Brain, Code2, Mic, GitCompare, Home } from 'lucide-react';

interface BreadcrumbItem {
  label: string;
  path?: string;
}

interface QuickLink {
  label: string;
  path: string;
  icon: React.ReactNode;
  description: string;
}

interface AIStudioNavProps {
  /** Current page name for breadcrumb */
  currentPage?: string;
  /** Show quick actions section */
  showQuickActions?: boolean;
}

export function AIStudioNav({ currentPage, showQuickActions = true }: AIStudioNavProps) {
  const pathname = window.location.pathname;

  // Build breadcrumbs based on current path
  const breadcrumbs = buildBreadcrumbs(pathname, currentPage);

  // Quick links to specialized tools
  const quickLinks: QuickLink[] = [
    {
      label: 'MockAI',
      path: '/mockai',
      icon: <Code2 className="w-4 h-4" />,
      description: 'Intelligent mock generation',
    },
    {
      label: 'Voice Interface',
      path: '/voice',
      icon: <Mic className="w-4 h-4" />,
      description: 'Voice commands & chat',
    },
    {
      label: 'Contract Diff',
      path: '/contract-diff',
      icon: <GitCompare className="w-4 h-4" />,
      description: 'Contract analysis & drift',
    },
  ];

  return (
    <div className="space-y-4">
      {/* Breadcrumb Navigation */}
      <nav className="flex items-center space-x-2 text-sm text-muted-foreground">
        <a
          href="/ai-studio"
          className="flex items-center hover:text-foreground transition-colors"
        >
          <Home className="w-4 h-4 mr-1" />
          AI Studio
        </a>
        {breadcrumbs.map((item, index) => (
          <React.Fragment key={index}>
            <ChevronRight className="w-4 h-4 text-muted-foreground" />
            {item.path ? (
              <a
                href={item.path}
                className="hover:text-foreground transition-colors"
              >
                {item.label}
              </a>
            ) : (
              <span className="text-foreground font-medium">
                {item.label}
              </span>
            )}
          </React.Fragment>
        ))}
      </nav>

      {/* Quick Actions Section */}
      {showQuickActions && (
        <div className="bg-muted rounded-lg p-4 border border-border">
          <div className="flex items-center space-x-2 mb-3">
            <Brain className="w-5 h-5 text-info-600 dark:text-info-400" />
            <h3 className="text-sm font-semibold text-foreground">
              Quick Actions
            </h3>
          </div>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
            {quickLinks.map((link) => (
              <a
                key={link.path}
                href={link.path}
                className="flex items-start space-x-3 p-3 rounded-md hover:bg-accent hover:text-accent-foreground transition-colors border border-border"
              >
                <div className="text-info-600 dark:text-info-400 mt-0.5">
                  {link.icon}
                </div>
                <div className="flex-1 min-w-0">
                  <div className="text-sm font-medium text-foreground">
                    {link.label}
                  </div>
                  <div className="text-xs text-muted-foreground mt-0.5">
                    {link.description}
                  </div>
                </div>
              </a>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

/**
 * Build breadcrumbs based on current path
 */
function buildBreadcrumbs(pathname: string, currentPage?: string): BreadcrumbItem[] {
  const items: BreadcrumbItem[] = [];

  // Map paths to breadcrumb labels
  const pathMap: Record<string, string> = {
    '/mockai': 'MockAI',
    '/voice': 'Voice Interface',
    '/contract-diff': 'Contract Diff',
    '/ai-studio': 'AI Studio',
  };

  // Split path and build breadcrumbs
  const pathParts = pathname.split('/').filter(Boolean);

  // If we're not on AI Studio home, add the current page
  if (pathname !== '/ai-studio' && pathname !== '/') {
    const pageName = currentPage || pathMap[pathname] || pathParts[pathParts.length - 1];
    items.push({
      label: pageName,
    });
  }

  return items;
}

/**
 * Back to AI Studio button component
 * Use this on sub-pages to provide navigation back to AI Studio
 */
export function BackToAIStudio() {
  return (
    <a
      href="/ai-studio"
      className="inline-flex items-center space-x-2 text-sm text-info-600 dark:text-info-400 hover:text-info-700 dark:hover:text-info-300 transition-colors"
    >
      <ChevronRight className="w-4 h-4 rotate-180" />
      <span>Back to AI Studio</span>
    </a>
  );
}
