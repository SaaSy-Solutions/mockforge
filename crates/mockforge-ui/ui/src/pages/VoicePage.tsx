//! Voice + LLM Interface Page
//!
//! This page provides a conversational interface for creating mocks using
//! natural language voice commands powered by LLM.

import React, { useState } from 'react';
import { VoiceInput, VoiceCommandResult } from '../components/voice/VoiceInput';
import { Card } from '../components/ui/Card';
import { Mic, Sparkles, FileCode } from 'lucide-react';

export function VoicePage() {
  const [history, setHistory] = useState<VoiceCommandResult[]>([]);

  const handleCommandProcessed = (result: VoiceCommandResult) => {
    setHistory(prev => [result, ...prev].slice(0, 10)); // Keep last 10 commands
  };

  return (
    <div className="container mx-auto p-6 space-y-6">
      {/* Header */}
      <div className="space-y-2">
        <div className="flex items-center gap-3">
          <Mic className="h-8 w-8 text-primary" />
          <h1 className="text-3xl font-bold">Voice + LLM Interface</h1>
        </div>
        <p className="text-muted-foreground">
          Build mocks conversationally using natural language commands powered by AI.
          Speak or type your requirements, and we'll generate an OpenAPI specification.
        </p>
      </div>

      {/* Features */}
      <div className="grid md:grid-cols-3 gap-4">
        <Card className="p-4">
          <div className="flex items-center gap-3 mb-2">
            <Mic className="h-5 w-5 text-primary" />
            <h3 className="font-semibold">Voice Input</h3>
          </div>
          <p className="text-sm text-muted-foreground">
            Use your microphone to speak commands naturally. Works with Chrome, Edge, and Safari.
          </p>
        </Card>
        <Card className="p-4">
          <div className="flex items-center gap-3 mb-2">
            <Sparkles className="h-5 w-5 text-primary" />
            <h3 className="font-semibold">AI-Powered</h3>
          </div>
          <p className="text-sm text-muted-foreground">
            LLM interprets your commands and extracts API requirements automatically.
          </p>
        </Card>
        <Card className="p-4">
          <div className="flex items-center gap-3 mb-2">
            <FileCode className="h-5 w-5 text-primary" />
            <h3 className="font-semibold">OpenAPI Output</h3>
          </div>
          <p className="text-sm text-muted-foreground">
            Generates valid OpenAPI 3.0 specifications ready to use with MockForge.
          </p>
        </Card>
      </div>

      {/* Voice Input Component */}
      <VoiceInput onCommandProcessed={handleCommandProcessed} />

      {/* Command History */}
      {history.length > 0 && (
        <div className="space-y-2">
          <h2 className="text-xl font-semibold">Recent Commands</h2>
          <div className="space-y-2">
            {history.map((item, index) => (
              <Card key={index} className="p-4">
                <div className="flex items-start justify-between">
                  <div className="flex-1">
                    <div className="font-medium mb-1">{item.command}</div>
                    <div className="text-sm text-muted-foreground">
                      {item.parsed.apiType} • {item.parsed.endpoints} endpoints • {item.parsed.models} models
                    </div>
                  </div>
                  {item.spec && (
                    <div className="text-xs text-muted-foreground">
                      {item.spec.title} v{item.spec.version}
                    </div>
                  )}
                </div>
              </Card>
            ))}
          </div>
        </div>
      )}

      {/* Examples */}
      <Card className="p-6">
        <h2 className="text-xl font-semibold mb-4">Example Commands</h2>
        <div className="grid md:grid-cols-2 gap-4">
          <div className="space-y-2">
            <div className="font-medium">Simple API</div>
            <div className="text-sm text-muted-foreground p-3 bg-muted rounded">
              "Create a todo API with endpoints for listing, creating, and updating tasks"
            </div>
          </div>
          <div className="space-y-2">
            <div className="font-medium">E-commerce</div>
            <div className="text-sm text-muted-foreground p-3 bg-muted rounded">
              "Create an e-commerce API with products, users, and a checkout flow"
            </div>
          </div>
          <div className="space-y-2">
            <div className="font-medium">With Models</div>
            <div className="text-sm text-muted-foreground p-3 bg-muted rounded">
              "Build a blog API with posts, comments, and user authentication"
            </div>
          </div>
          <div className="space-y-2">
            <div className="font-medium">Complex</div>
            <div className="text-sm text-muted-foreground p-3 bg-muted rounded">
              "Create a social media API with users, posts, likes, and a feed endpoint"
            </div>
          </div>
        </div>
      </Card>
    </div>
  );
}
