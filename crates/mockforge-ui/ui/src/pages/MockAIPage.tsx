//! MockAI Main Page
//!
//! Landing page for MockAI features with links to all capabilities,
//! recent activity, and quick actions.

import React, { useState, useEffect } from 'react';
import {
  Brain,
  Code2,
  BarChart3,
  FileText,
  Sparkles,
  ArrowRight,
  TrendingUp,
  Zap,
  BookOpen,
  PlayCircle,
} from 'lucide-react';
import { PageHeader, Card, Button, Badge, Section } from '../components/ui/DesignSystem';
import { apiService } from '../services/api';
import { toast } from 'sonner';
import { logger } from '@/utils/logger';

interface FeatureCardProps {
  title: string;
  description: string;
  icon: React.ReactNode;
  link: string;
  badge?: string;
  onClick?: () => void;
}

function FeatureCard({
  title,
  description,
  icon,
  link,
  badge,
  onClick,
}: FeatureCardProps) {
  const handleClick = () => {
    if (onClick) {
      onClick();
    } else {
      // Navigate to the feature page
      window.location.hash = link;
    }
  };

  return (
    <Card
      className="p-6 hover:shadow-lg transition-shadow cursor-pointer"
      onClick={handleClick}
    >
      <div className="flex items-start justify-between mb-4">
        <div className="p-3 bg-blue-100 dark:bg-blue-900 rounded-lg">{icon}</div>
        {badge && <Badge variant="default">{badge}</Badge>}
      </div>
      <h3 className="text-lg font-semibold mb-2">{title}</h3>
      <p className="text-sm text-gray-600 dark:text-gray-400 mb-4">{description}</p>
      <div className="flex items-center text-sm text-blue-600 dark:text-blue-400">
        <span>Learn more</span>
        <ArrowRight className="h-4 w-4 ml-1" />
      </div>
    </Card>
  );
}

export function MockAIPage() {
  const [stats, setStats] = useState<{
    rulesCount: number;
    openApiGenerated: boolean;
    lastGenerated?: string;
  } | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const fetchStats = async () => {
      try {
        // Fetch rule explanations count
        const rulesResponse = await apiService.listRuleExplanations();
        setStats({
          rulesCount: rulesResponse.total,
          openApiGenerated: false, // TODO: Track OpenAPI generation status
          lastGenerated: undefined,
        });
      } catch (err) {
        logger.error('Failed to fetch MockAI stats', err);
        // Don't show error, just use defaults
        setStats({
          rulesCount: 0,
          openApiGenerated: false,
        });
      } finally {
        setLoading(false);
      }
    };

    fetchStats();
  }, []);

  const features = [
    {
      title: 'OpenAPI Generation',
      description:
        'Generate OpenAPI 3.0 specifications from recorded HTTP traffic using AI-powered pattern detection',
      icon: <FileText className="h-6 w-6 text-blue-600 dark:text-blue-400" />,
      link: 'mockai-openapi-generator',
      badge: 'New',
    },
    {
      title: 'Rules Dashboard',
      description:
        'View and explore all generated behavioral rules with detailed explanations and confidence scores',
      icon: <BarChart3 className="h-6 w-6 text-blue-600 dark:text-blue-400" />,
      link: 'mockai-rules',
    },
    {
      title: 'Intelligent Responses',
      description:
        'Generate context-aware mock responses using LLM-powered decision making',
      icon: <Sparkles className="h-6 w-6 text-blue-600 dark:text-blue-400" />,
      link: '#', // TODO: Add intelligent responses page
      badge: 'Coming Soon',
    },
    {
      title: 'Learn from Examples',
      description:
        'Train MockAI to understand your API patterns from example request/response pairs',
      icon: <Brain className="h-6 w-6 text-blue-600 dark:text-blue-400" />,
      link: '#', // TODO: Add learn page
      badge: 'Coming Soon',
    },
  ];

  return (
    <div className="space-y-6">
      <PageHeader
        title="MockAI"
        description="AI-powered mock API intelligence for realistic, context-aware responses"
        icon={<Brain className="h-6 w-6" />}
      />

      {/* Stats Overview */}
      {!loading && stats && (
        <Card className="p-6">
          <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
            <div className="text-center">
              <div className="text-3xl font-bold text-blue-600 dark:text-blue-400 mb-2">
                {stats.rulesCount}
              </div>
              <div className="text-sm text-gray-600 dark:text-gray-400">
                Generated Rules
              </div>
            </div>
            <div className="text-center">
              <div className="text-3xl font-bold text-green-600 dark:text-green-400 mb-2">
                {stats.openApiGenerated ? 'Yes' : 'No'}
              </div>
              <div className="text-sm text-gray-600 dark:text-gray-400">
                OpenAPI Specs Generated
              </div>
            </div>
            <div className="text-center">
              <div className="text-3xl font-bold text-purple-600 dark:text-purple-400 mb-2">
                <Zap className="h-8 w-8 mx-auto" />
              </div>
              <div className="text-sm text-gray-600 dark:text-gray-400">
                AI-Powered
              </div>
            </div>
          </div>
        </Card>
      )}

      {/* Quick Actions */}
      <Section title="Quick Actions">
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <Button
            variant="primary"
            onClick={() => {
              window.location.hash = 'mockai-openapi-generator';
            }}
            className="w-full"
          >
            <FileText className="h-4 w-4 mr-2" />
            Generate OpenAPI from Traffic
          </Button>
          <Button
            variant="primary"
            onClick={() => {
              window.location.hash = 'mockai-rules';
            }}
            className="w-full"
          >
            <BarChart3 className="h-4 w-4 mr-2" />
            View Rules Dashboard
          </Button>
          <Button
            variant="outline"
            onClick={() => {
              // TODO: Open learn from examples dialog/modal
              toast.info('Use the API endpoint POST /__mockforge/api/mockai/learn to learn from examples');
            }}
            className="w-full"
          >
            <Brain className="h-4 w-4 mr-2" />
            Learn from Examples
          </Button>
        </div>
      </Section>

      {/* Features Grid */}
      <Section title="Features">
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          {features.map((feature) => (
            <FeatureCard key={feature.title} {...feature} />
          ))}
        </div>
      </Section>

      {/* Getting Started */}
      <Card className="p-6 bg-gradient-to-r from-blue-50 to-purple-50 dark:from-blue-900/20 dark:to-purple-900/20">
        <div className="flex items-start gap-4">
          <div className="p-3 bg-blue-100 dark:bg-blue-900 rounded-lg">
            <BookOpen className="h-6 w-6 text-blue-600 dark:text-blue-400" />
          </div>
          <div className="flex-1">
            <h3 className="text-lg font-semibold mb-2">Getting Started</h3>
            <p className="text-sm text-gray-700 dark:text-gray-300 mb-4">
              Start using MockAI to enhance your mock APIs with intelligent behavior:
            </p>
            <ol className="list-decimal list-inside space-y-2 text-sm text-gray-700 dark:text-gray-300">
              <li>
                Record API traffic using the{' '}
                <a
                  href="#recorder"
                  className="text-blue-600 dark:text-blue-400 hover:underline"
                >
                  API Flight Recorder
                </a>
              </li>
              <li>
                Generate OpenAPI specs from recorded traffic using the{' '}
                <a
                  href="#mockai-openapi-generator"
                  className="text-blue-600 dark:text-blue-400 hover:underline"
                >
                  OpenAPI Generator
                </a>
              </li>
              <li>
                View and understand generated rules in the{' '}
                <a
                  href="#mockai-rules"
                  className="text-blue-600 dark:text-blue-400 hover:underline"
                >
                  Rules Dashboard
                </a>
              </li>
              <li>
                Learn from examples to train MockAI on your API patterns
              </li>
            </ol>
          </div>
        </div>
      </Card>

      {/* Documentation Links */}
      <Card className="p-6">
        <h3 className="text-lg font-semibold mb-4 flex items-center gap-2">
          <BookOpen className="h-5 w-5" />
          Documentation & Resources
        </h3>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div className="p-4 border rounded-lg hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors cursor-pointer">
            <div className="flex items-center gap-2 mb-2">
              <FileText className="h-4 w-4 text-blue-600 dark:text-blue-400" />
              <div className="font-medium">OpenAPI Generation Guide</div>
            </div>
            <div className="text-sm text-gray-600 dark:text-gray-400">
              Learn how to generate OpenAPI specs from recorded traffic with AI-powered pattern detection
            </div>
          </div>
          <div className="p-4 border rounded-lg hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors cursor-pointer">
            <div className="flex items-center gap-2 mb-2">
              <BarChart3 className="h-4 w-4 text-purple-600 dark:text-purple-400" />
              <div className="font-medium">Rule Explanations Guide</div>
            </div>
            <div className="text-sm text-gray-600 dark:text-gray-400">
              Understand how MockAI generates and explains behavioral rules with confidence scores
            </div>
          </div>
        </div>
      </Card>

      {/* Tips & Best Practices */}
      <Card className="p-6 bg-blue-50 dark:bg-blue-900/20 border-blue-200 dark:border-blue-800">
        <h3 className="text-lg font-semibold mb-4 flex items-center gap-2">
          <Zap className="h-5 w-5 text-blue-600 dark:text-blue-400" />
          Tips & Best Practices
        </h3>
        <div className="space-y-3 text-sm">
          <div className="flex items-start gap-3">
            <div className="w-6 h-6 rounded-full bg-blue-100 dark:bg-blue-900 flex items-center justify-center flex-shrink-0 mt-0.5">
              <span className="text-xs font-bold text-blue-600 dark:text-blue-400">1</span>
            </div>
            <div>
              <div className="font-medium text-gray-900 dark:text-gray-100">
                Record Comprehensive Traffic
              </div>
              <div className="text-gray-700 dark:text-gray-300">
                Capture diverse examples of your API endpoints to improve pattern detection and confidence scores
              </div>
            </div>
          </div>
          <div className="flex items-start gap-3">
            <div className="w-6 h-6 rounded-full bg-blue-100 dark:bg-blue-900 flex items-center justify-center flex-shrink-0 mt-0.5">
              <span className="text-xs font-bold text-blue-600 dark:text-blue-400">2</span>
            </div>
            <div>
              <div className="font-medium text-gray-900 dark:text-gray-100">
                Review Confidence Scores
              </div>
              <div className="text-gray-700 dark:text-gray-300">
                High confidence rules (≥80%) are more reliable. Review and validate lower confidence rules manually
              </div>
            </div>
          </div>
          <div className="flex items-start gap-3">
            <div className="w-6 h-6 rounded-full bg-blue-100 dark:bg-blue-900 flex items-center justify-center flex-shrink-0 mt-0.5">
              <span className="text-xs font-bold text-blue-600 dark:text-blue-400">3</span>
            </div>
            <div>
              <div className="font-medium text-gray-900 dark:text-gray-100">
                Use Time Filters
              </div>
              <div className="text-gray-700 dark:text-gray-300">
                Filter by time range to focus on recent API changes and maintain up-to-date specifications
              </div>
            </div>
          </div>
        </div>
      </Card>
    </div>
  );
}
