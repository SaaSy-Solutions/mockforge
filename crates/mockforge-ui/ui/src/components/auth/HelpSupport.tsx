import { logger } from '@/utils/logger';
import { useState } from 'react';
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
    { keys: `${modKey} + /`, description: 'Show keyboard shortcuts' },
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
      <DialogContent className="sm:max-w-3xl max-h-[90vh] overflow-y-auto bg-white dark:bg-gray-900">
        <DialogHeader className="space-y-2">
          <DialogTitle className="flex items-center gap-2 text-xl font-semibold text-gray-900 dark:text-gray-100">
            <HelpCircle className="h-5 w-5" />
            Help & Support
          </DialogTitle>
          <DialogDescription className="text-sm text-gray-600 dark:text-gray-400 leading-relaxed">
            Learn how to use MockForge effectively
          </DialogDescription>
          <DialogClose onClick={() => onOpenChange(false)} />
        </DialogHeader>

        <TabsProvider value={activeTab} onValueChange={setActiveTab}>
          <Tabs value={activeTab} onValueChange={setActiveTab} className="w-full">
            <TabsList className="grid w-full grid-cols-3 bg-gray-100 dark:bg-gray-800">
              <TabsTrigger value="quickstart" className="flex items-center gap-2 text-gray-700 dark:text-gray-300 data-[state=active]:text-gray-900 dark:data-[state=active]:text-gray-100 data-[state=active]:bg-white dark:data-[state=active]:bg-gray-700">
                <Rocket className="h-4 w-4" />
                <span className="hidden sm:inline">Quick Start</span>
              </TabsTrigger>
              <TabsTrigger value="shortcuts" className="flex items-center gap-2 text-gray-700 dark:text-gray-300 data-[state=active]:text-gray-900 dark:data-[state=active]:text-gray-100 data-[state=active]:bg-white dark:data-[state=active]:bg-gray-700">
                <Keyboard className="h-4 w-4" />
                <span className="hidden sm:inline">Shortcuts</span>
              </TabsTrigger>
              <TabsTrigger value="faq" className="flex items-center gap-2 text-gray-700 dark:text-gray-300 data-[state=active]:text-gray-900 dark:data-[state=active]:text-gray-100 data-[state=active]:bg-white dark:data-[state=active]:bg-gray-700">
                <MessageCircle className="h-4 w-4" />
                <span className="hidden sm:inline">FAQ</span>
              </TabsTrigger>
            </TabsList>

            <TabsContent value="quickstart" className="space-y-4 mt-4">
              <div className="space-y-6">
                <div>
                  <h3 className="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-3">
                    Welcome to MockForge!
                  </h3>
                  <p className="text-sm text-gray-600 dark:text-gray-400 mb-4">
                    MockForge is a powerful API mocking and testing platform. Here's how to get started:
                  </p>
                </div>

                <div className="space-y-4">
                  <div className="flex gap-3">
                    <div className="flex-shrink-0 w-8 h-8 bg-orange-100 dark:bg-orange-900 text-orange-600 dark:text-orange-300 rounded-full flex items-center justify-center font-semibold">
                      1
                    </div>
                    <div>
                      <h4 className="font-medium text-gray-900 dark:text-gray-100 mb-1">Create a Workspace</h4>
                      <p className="text-sm text-gray-600 dark:text-gray-400">
                        Start by creating a workspace to organize your mocks. Navigate to Workspaces and click "New Workspace".
                      </p>
                    </div>
                  </div>

                  <div className="flex gap-3">
                    <div className="flex-shrink-0 w-8 h-8 bg-orange-100 dark:bg-orange-900 text-orange-600 dark:text-orange-300 rounded-full flex items-center justify-center font-semibold">
                      2
                    </div>
                    <div>
                      <h4 className="font-medium text-gray-900 dark:text-gray-100 mb-1">Import or Create Fixtures</h4>
                      <p className="text-sm text-gray-600 dark:text-gray-400">
                        Import fixtures from OpenAPI/Swagger specs or create them manually in the Fixtures page.
                      </p>
                    </div>
                  </div>

                  <div className="flex gap-3">
                    <div className="flex-shrink-0 w-8 h-8 bg-orange-100 dark:bg-orange-900 text-orange-600 dark:text-orange-300 rounded-full flex items-center justify-center font-semibold">
                      3
                    </div>
                    <div>
                      <h4 className="font-medium text-gray-900 dark:text-gray-100 mb-1">Configure Services</h4>
                      <p className="text-sm text-gray-600 dark:text-gray-400">
                        Set up your mock services with specific routes, responses, and behaviors in the Services page.
                      </p>
                    </div>
                  </div>

                  <div className="flex gap-3">
                    <div className="flex-shrink-0 w-8 h-8 bg-orange-100 dark:bg-orange-900 text-orange-600 dark:text-orange-300 rounded-full flex items-center justify-center font-semibold">
                      4
                    </div>
                    <div>
                      <h4 className="font-medium text-gray-900 dark:text-gray-100 mb-1">Monitor & Test</h4>
                      <p className="text-sm text-gray-600 dark:text-gray-400">
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
                  <h3 className="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-3">
                    Keyboard Shortcuts
                  </h3>
                  <p className="text-sm text-gray-600 dark:text-gray-400 mb-4">
                    Use these shortcuts to navigate faster:
                  </p>
                </div>

                <div className="space-y-2">
                  {shortcuts.map((shortcut, index) => (
                    <div
                      key={index}
                      className="flex items-center justify-between p-3 bg-gray-50 dark:bg-gray-800 rounded-lg"
                    >
                      <span className="text-sm text-gray-600 dark:text-gray-400">
                        {shortcut.description}
                      </span>
                      <kbd className="px-3 py-1 text-sm font-mono bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded shadow-sm text-gray-900 dark:text-gray-100">
                        {shortcut.keys}
                      </kbd>
                    </div>
                  ))}
                </div>

                <div className="mt-6 p-4 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg">
                  <p className="text-sm text-blue-800 dark:text-blue-200">
                    💡 Tip: You can enable/disable keyboard shortcuts in Preferences
                  </p>
                </div>
              </div>
            </TabsContent>

            <TabsContent value="faq" className="space-y-4 mt-4">
              <div className="space-y-4">
                <div>
                  <h3 className="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-3">
                    Frequently Asked Questions
                  </h3>
                </div>

                <div className="space-y-4">
                  {faqs.map((faq, index) => (
                    <div
                      key={index}
                      className="p-4 bg-gray-50 dark:bg-gray-800 rounded-lg"
                    >
                      <h4 className="font-medium text-gray-900 dark:text-gray-100 mb-2">
                        {faq.question}
                      </h4>
                      <p className="text-sm text-gray-600 dark:text-gray-400">
                        {faq.answer}
                      </p>
                    </div>
                  ))}
                </div>
              </div>
            </TabsContent>
          </Tabs>
        </TabsProvider>

        <DialogFooter className="flex items-center justify-between border-t border-gray-200 dark:border-gray-700 pt-4 mt-4">
          <div className="flex items-center gap-4 text-sm">
            <a
              href="https://github.com/SaaSy-Solutions/mockforge"
              target="_blank"
              rel="noopener noreferrer"
              className="flex items-center gap-1 text-gray-600 dark:text-gray-400 hover:text-orange-600 dark:hover:text-orange-400 transition-colors"
            >
              <ExternalLink className="h-4 w-4" />
              GitHub
            </a>
            <a
              href="https://docs.mockforge.dev/api/admin-ui-rest.html"
              target="_blank"
              rel="noopener noreferrer"
              className="flex items-center gap-1 text-gray-600 dark:text-gray-400 hover:text-orange-600 dark:hover:text-orange-400 transition-colors"
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
