import { Logo } from '../ui/Logo';

export function Sidebar() {
  return (
    <aside className="w-64 flex-shrink-0 border-r bg-background p-6">
      <div className="flex h-full flex-col">
        <div className="mb-8">
          <Logo variant="full" size="lg" />
        </div>
        <nav className="flex flex-col space-y-2">
          {/* Navigation items can be moved here later if needed */}
        </nav>
      </div>
    </aside>
  );
}
