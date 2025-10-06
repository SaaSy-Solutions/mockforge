import { logger } from '@/utils/logger';
import { Button } from '../ui/button';
import { UserProfile } from '../auth/UserProfile';

interface HeaderProps {
  onRefresh?: () => void;
}

export function Header({ onRefresh }: HeaderProps) {
  return (
    <header className="border-b bg-background px-6 py-4">
      <div className="flex items-center justify-end">
        <div className="flex items-center space-x-4">
          <Button variant="outline" size="sm" onClick={onRefresh}>
            Refresh
          </Button>
          <UserProfile />
        </div>
      </div>
    </header>
  );
}
