//! AI Studio Navigation Component
//!
//! Provides breadcrumb navigation and quick links to specialized AI tools.
//! This component ties together AI Studio (front door) with deep-linked specialized pages.

import React from 'react';
import { Link, useLocation } from 'react-router-dom';
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
  const location = useLocation();

  // Build breadcrumbs based on current path
  const breadcrumbs = buildBreadcrumbs(location.pathname, currentPage);

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
      <nav className="flex items-center space-x-2 text-sm text-gray-600 dark:text-gray-400">
        <Link
          to="/ai-studio"
          className="flex items-center hover:text-gray-900 dark:hover:text-gray-100 transition-colors"
        >
          <Home className="w-4 h-4 mr-1" />
          AI Studio
        </Link>
        {breadcrumbs.map((item, index) => (
          <React.Fragment key={index}>
            <ChevronRight className="w-4 h-4 text-gray-400" />
            {item.path ? (
              <Link
                to={item.path}
                className="hover:text-gray-900 dark:hover:text-gray-100 transition-colors"
              >
                {item.label}
              </Link>
            ) : (
              <span className="text-gray-900 dark:text-gray-100 font-medium">
                {item.label}
              </span>
            )}
          </React.Fragment>
        ))}
      </nav>

      {/* Quick Actions Section */}
      {showQuickActions && (
        <div className="bg-gray-50 dark:bg-gray-800 rounded-lg p-4 border border-gray-200 dark:border-gray-700">
          <div className="flex items-center space-x-2 mb-3">
            <Brain className="w-5 h-5 text-blue-600 dark:text-blue-400" />
            <h3 className="text-sm font-semibold text-gray-900 dark:text-gray-100">
              Quick Actions
            </h3>
          </div>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
            {quickLinks.map((link) => (
              <Link
                key={link.path}
                to={link.path}
                className="flex items-start space-x-3 p-3 rounded-md hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors border border-gray-200 dark:border-gray-700"
              >
                <div className="text-blue-600 dark:text-blue-400 mt-0.5">
                  {link.icon}
                </div>
                <div className="flex-1 min-w-0">
                  <div className="text-sm font-medium text-gray-900 dark:text-gray-100">
                    {link.label}
                  </div>
                  <div className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                    {link.description}
                  </div>
                </div>
              </Link>
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
    <Link
      to="/ai-studio"
      className="inline-flex items-center space-x-2 text-sm text-blue-600 dark:text-blue-400 hover:text-blue-700 dark:hover:text-blue-300 transition-colors"
    >
      <ChevronRight className="w-4 h-4 rotate-180" />
      <span>Back to AI Studio</span>
    </Link>
  );
}
