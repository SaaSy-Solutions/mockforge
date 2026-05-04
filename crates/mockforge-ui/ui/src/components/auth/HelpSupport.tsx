import { useState } from 'react';
import { Link } from 'react-router-dom';
import { Button } from '../ui/button';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
  DialogClose,
} from '../ui/Dialog';
import {
  Tabs,
  TabsProvider,
  TabsList,
  TabsTrigger,
  TabsContent,
} from '../ui/Tabs';
import {
  HelpCircle,
  Rocket,
  Keyboard,
  MessageCircle,
  ExternalLink,
  Book,
} from 'lucide-react';

interface HelpSupportProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function HelpSupport({ open, onOpenChange }: HelpSupportProps) {
  const [activeTab, setActiveTab] = useState('quickstart');
  const isMac = navigator.userAgent.toUpperCase().indexOf('MAC') >= 0;
  const modKey = isMac ? '⌘' : 'Ctrl';

  const shortcuts = [
    { keys: `${modKey} + K`, description: 'Focus global search' },
    { keys: 'Esc', description: 'Clear search / Close dialogs' },
    { keys: 'Shift + ?', description: 'Open Help & Support' },
  ];

  const faqs = [
    {
      question: 'How do I create a new workspace?',
      answer: 'Navigate to the Workspaces page and click the "New Workspace" button. Fill in the required details and click "Create".',
    },
    {
      question: 'How do I import fixtures from OpenAPI/Swagger?',
      answer: 'Go to the Import page, select "OpenAPI/Swagger" as the source, upload your spec file or provide a URL, and click "Import".',
    },
    {
      question: 'What are chains and how do I use them?',
      answer: 'Chains allow you to link multiple mock responses together in sequence. Create a chain in the Chains page and define the order of responses.',
    },
    {
      question: 'How do I view real-time logs?',
      answer: 'Visit the Logs page where you can see live request/response logs. Use filters to narrow down specific requests or services.',
    },
    {
      question: 'Can I export my workspace configuration?',
      answer: 'Yes! Go to the Workspaces page, select your workspace, and use the export option to download your configuration.',
    },
  ];

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-3xl max-h-[90vh] overflow-y-auto bg-card">
        <DialogHeader className="space-y-2">
          <DialogTitle className="flex items-center gap-2 text-xl font-semibold text-foreground">
            <HelpCircle className="h-5 w-5" />
            Help & Support
          </DialogTitle>
          <DialogDescription className="text-sm text-muted-foreground leading-relaxed">
            Learn how to use MockForge effectively
          </DialogDescription>
          <DialogClose onClick={() => onOpenChange(false)} />
        </DialogHeader>

        <TabsProvider value={activeTab} onValueChange={setActiveTab}>
          <Tabs value={activeTab} onValueChange={setActiveTab} className="w-full">
            <TabsList className="grid w-full grid-cols-3 bg-muted">
              <TabsTrigger value="quickstart" className="flex items-center gap-2 text-foreground data-[state=active]:text-foreground data-[state=active]:bg-card">
                <Rocket className="h-4 w-4" />
                <span className="hidden sm:inline">Quick Start</span>
              </TabsTrigger>
              <TabsTrigger value="shortcuts" className="flex items-center gap-2 text-foreground data-[state=active]:text-foreground data-[state=active]:bg-card">
                <Keyboard className="h-4 w-4" />
                <span className="hidden sm:inline">Shortcuts</span>
              </TabsTrigger>
              <TabsTrigger value="faq" className="flex items-center gap-2 text-foreground data-[state=active]:text-foreground data-[state=active]:bg-card">
                <MessageCircle className="h-4 w-4" />
                <span className="hidden sm:inline">FAQ</span>
              </TabsTrigger>
            </TabsList>

            <TabsContent value="quickstart" className="space-y-4 mt-4">
              <div className="space-y-6">
                <div>
                  <h3 className="text-lg font-semibold text-foreground mb-3">
                    Welcome to MockForge!
                  </h3>
                  <p className="text-sm text-muted-foreground mb-4">
                    MockForge is a powerful API mocking and testing platform. Here's how to get started:
                  </p>
                </div>

                <div className="space-y-4">
                  <div className="flex gap-3">
                    <div className="flex-shrink-0 w-8 h-8 bg-orange-100 dark:bg-orange-900 text-orange-600 dark:text-orange-300 rounded-full flex items-center justify-center font-semibold">
                      1
                    </div>
                    <div>
                      <h4 className="font-medium text-foreground mb-1">Create a Workspace</h4>
                      <p className="text-sm text-muted-foreground">
                        Start by creating a workspace to organize your mocks. Navigate to Workspaces and click "New Workspace".
                      </p>
                    </div>
                  </div>

                  <div className="flex gap-3">
                    <div className="flex-shrink-0 w-8 h-8 bg-orange-100 dark:bg-orange-900 text-orange-600 dark:text-orange-300 rounded-full flex items-center justify-center font-semibold">
                      2
                    </div>
                    <div>
                      <h4 className="font-medium text-foreground mb-1">Import or Create Fixtures</h4>
                      <p className="text-sm text-muted-foreground">
                        Import fixtures from OpenAPI/Swagger specs or create them manually in the Fixtures page.
                      </p>
                    </div>
                  </div>

                  <div className="flex gap-3">
                    <div className="flex-shrink-0 w-8 h-8 bg-orange-100 dark:bg-orange-900 text-orange-600 dark:text-orange-300 rounded-full flex items-center justify-center font-semibold">
                      3
                    </div>
                    <div>
                      <h4 className="font-medium text-foreground mb-1">Configure Services</h4>
                      <p className="text-sm text-muted-foreground">
                        Set up your mock services with specific routes, responses, and behaviors in the Services page.
                      </p>
                    </div>
                  </div>

                  <div className="flex gap-3">
                    <div className="flex-shrink-0 w-8 h-8 bg-orange-100 dark:bg-orange-900 text-orange-600 dark:text-orange-300 rounded-full flex items-center justify-center font-semibold">
                      4
                    </div>
                    <div>
                      <h4 className="font-medium text-foreground mb-1">Monitor & Test</h4>
                      <p className="text-sm text-muted-foreground">
                        Use the Logs and Metrics pages to monitor requests and the Testing page to validate your mocks.
                      </p>
                    </div>
                  </div>
                </div>
              </div>
            </TabsContent>

            <TabsContent value="shortcuts" className="space-y-4 mt-4">
              <div className="space-y-4">
                <div>
                  <h3 className="text-lg font-semibold text-foreground mb-3">
                    Keyboard Shortcuts
                  </h3>
                  <p className="text-sm text-muted-foreground mb-4">
                    Use these shortcuts to navigate faster:
                  </p>
                </div>

                <div className="space-y-2">
                  {shortcuts.map((shortcut, index) => (
                    <div
                      key={index}
                      className="flex items-center justify-between p-3 bg-muted rounded-lg"
                    >
                      <span className="text-sm text-muted-foreground">
                        {shortcut.description}
                      </span>
                      <kbd className="px-3 py-1 text-sm font-mono bg-card dark:bg-gray-700 border border-border rounded shadow-sm text-foreground">
                        {shortcut.keys}
                      </kbd>
                    </div>
                  ))}
                </div>

                <div className="mt-6 p-4 bg-info-50 dark:bg-info-900/20 border border-info-200 dark:border-info-800 rounded-lg">
                  <p className="text-sm text-info-700 dark:text-info-200">
                    💡 Tip: You can enable/disable keyboard shortcuts in Preferences
                  </p>
                </div>
              </div>
            </TabsContent>

            <TabsContent value="faq" className="space-y-4 mt-4">
              <div className="space-y-4">
                <div>
                  <h3 className="text-lg font-semibold text-foreground mb-3">
                    Frequently Asked Questions
                  </h3>
                </div>

                <div className="space-y-4">
                  {faqs.map((faq, index) => (
                    <div
                      key={index}
                      className="p-4 bg-muted rounded-lg"
                    >
                      <h4 className="font-medium text-foreground mb-2">
                        {faq.question}
                      </h4>
                      <p className="text-sm text-muted-foreground">
                        {faq.answer}
                      </p>
                    </div>
                  ))}
                </div>
              </div>
            </TabsContent>
          </Tabs>
        </TabsProvider>

        <DialogFooter className="flex items-center justify-between border-t border-border pt-4 mt-4 flex-wrap gap-y-2">
          <div className="flex items-center gap-4 text-sm flex-wrap">
            <Link
              to="/faq"
              onClick={() => onOpenChange(false)}
              className="flex items-center gap-1 text-muted-foreground hover:text-orange-600 dark:hover:text-orange-400 transition-colors"
            >
              <MessageCircle className="h-4 w-4" />
              Full FAQ
            </Link>
            <Link
              to="/support"
              onClick={() => onOpenChange(false)}
              className="flex items-center gap-1 text-muted-foreground hover:text-orange-600 dark:hover:text-orange-400 transition-colors"
            >
              <HelpCircle className="h-4 w-4" />
              Contact support
            </Link>
            <Link
              to="/terms"
              onClick={() => onOpenChange(false)}
              className="flex items-center gap-1 text-muted-foreground hover:text-orange-600 dark:hover:text-orange-400 transition-colors"
            >
              Terms
            </Link>
            <Link
              to="/privacy"
              onClick={() => onOpenChange(false)}
              className="flex items-center gap-1 text-muted-foreground hover:text-orange-600 dark:hover:text-orange-400 transition-colors"
            >
              Privacy
            </Link>
            <a
              href="https://github.com/SaaSy-Solutions/mockforge"
              target="_blank"
              rel="noopener noreferrer"
              className="flex items-center gap-1 text-muted-foreground hover:text-orange-600 dark:hover:text-orange-400 transition-colors"
            >
              <ExternalLink className="h-4 w-4" />
              GitHub
            </a>
            <a
              href="https://docs.mockforge.dev/api/admin-ui-rest.html"
              target="_blank"
              rel="noopener noreferrer"
              className="flex items-center gap-1 text-muted-foreground hover:text-orange-600 dark:hover:text-orange-400 transition-colors"
            >
              <Book className="h-4 w-4" />
              API Docs
            </a>
          </div>
          <Button
            type="button"
            onClick={() => onOpenChange(false)}
          >
            Close
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
