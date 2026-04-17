/**
 * MarketplaceTabs
 *
 * Shared tab bar for the three marketplace pages — Templates, Scenarios, Plugins.
 * Rendered at the top of each page so users can switch without leaving the
 * content area. Deep links (`/template-marketplace`, `/scenario-marketplace`,
 * `/plugin-registry`) continue to work.
 */

import React from 'react';
import { Tabs, Tab, Box } from '@mui/material';
import {
  Description as TemplateIcon,
  Movie as ScenarioIcon,
  Extension as PluginIcon,
} from '@mui/icons-material';
import { useLocation, useNavigate } from 'react-router-dom';

const tabs: Array<{ path: string; label: string; icon: React.ReactElement }> = [
  { path: '/template-marketplace', label: 'Templates', icon: <TemplateIcon fontSize="small" /> },
  { path: '/scenario-marketplace', label: 'Scenarios', icon: <ScenarioIcon fontSize="small" /> },
  { path: '/plugin-registry', label: 'Plugins', icon: <PluginIcon fontSize="small" /> },
];

export const MarketplaceTabs: React.FC = () => {
  const location = useLocation();
  const navigate = useNavigate();
  const current = tabs.findIndex((t) => t.path === location.pathname);

  return (
    <Box sx={{ borderBottom: 1, borderColor: 'divider', mb: 3 }}>
      <Tabs
        value={current === -1 ? 0 : current}
        onChange={(_, idx) => navigate(tabs[idx].path)}
        aria-label="Marketplace sections"
      >
        {tabs.map((t) => (
          <Tab
            key={t.path}
            label={t.label}
            icon={t.icon}
            iconPosition="start"
            sx={{ minHeight: 48 }}
          />
        ))}
      </Tabs>
    </Box>
  );
};

export default MarketplaceTabs;
