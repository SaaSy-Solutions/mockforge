/** Shared route navigation for the three marketplace pages. */
import { Clapperboard, FileText, Puzzle } from 'lucide-react';
import { NavLink, useLocation } from 'react-router-dom';
import { Button } from '../ui/button';
import { cn } from '../../utils/cn';

const tabs = [
  { path: '/template-marketplace', label: 'Templates', icon: FileText },
  { path: '/scenario-marketplace', label: 'Scenarios', icon: Clapperboard },
  { path: '/plugin-registry', label: 'Plugins', icon: Puzzle },
];

export function MarketplaceTabs() {
  const { pathname } = useLocation();
  return (
    <nav aria-label="Marketplace sections" className="mb-6 overflow-x-auto border-b border-border">
      <ul className="flex w-max min-w-full list-none">
        {tabs.map(({ path, label, icon: Icon }) => (
          <li key={path}>
            <Button asChild variant="ghost" className={cn(
              'h-12 gap-2 rounded-none border-b-2 px-4',
              pathname === path
                ? 'border-primary text-primary'
                : 'border-transparent text-muted-foreground',
            )}>
              <NavLink to={path} end>
                <Icon className="h-4 w-4" aria-hidden="true" />
                {label}
              </NavLink>
            </Button>
          </li>
        ))}
      </ul>
    </nav>
  );
}

export default MarketplaceTabs;
