//! Voice Input Component using Web Speech API
//!
//! This component provides voice input functionality for the MockForge Admin UI,
//! allowing users to create mocks conversationally using natural language.

import React, { useState, useEffect, useRef } from 'react';
import { Mic, MicOff, Loader2, CheckCircle2, XCircle, Download, Play } from 'lucide-react';
import { Button } from '../ui/button';
import { cn } from '../../utils/cn';

interface VoiceInputProps {
  onCommandProcessed?: (result: VoiceCommandResult) => void;
  className?: string;
}

export interface VoiceCommandResult {
  command: string;
  parsed: {
    apiType: string;
    title: string;
    description: string;
    endpoints: number;
    models: number;
  };
  spec?: {
    title: string;
    version: string;
    json: string;
  };
  error?: string;
}

export function VoiceInput({ onCommandProcessed, className }: VoiceInputProps) {
  const [isListening, setIsListening] = useState(false);
  const [transcript, setTranscript] = useState('');
  const [isProcessing, setIsProcessing] = useState(false);
  const [result, setResult] = useState<VoiceCommandResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isSupported, setIsSupported] = useState(false);

  const recognitionRef = useRef<SpeechRecognition | null>(null);
  const finalTranscriptRef = useRef('');

  // Check if Web Speech API is supported
  useEffect(() => {
    const SpeechRecognition =
      (window as any).SpeechRecognition ||
      (window as any).webkitSpeechRecognition;

    setIsSupported(!!SpeechRecognition);

    if (SpeechRecognition) {
      const recognition = new SpeechRecognition();
      recognition.continuous = false;
      recognition.interimResults = true;
      recognition.lang = 'en-US';

      recognition.onstart = () => {
        setIsListening(true);
        setError(null);
        finalTranscriptRef.current = '';
      };

      recognition.onresult = (event: SpeechRecognitionEvent) => {
        let interimTranscript = '';
        let finalTranscript = '';

        for (let i = event.resultIndex; i < event.results.length; i++) {
          const transcript = event.results[i][0].transcript;
          if (event.results[i].isFinal) {
            finalTranscript += transcript + ' ';
          } else {
            interimTranscript += transcript;
          }
        }

        finalTranscriptRef.current += finalTranscript;
        setTranscript(finalTranscriptRef.current + interimTranscript);
      };

      recognition.onerror = (event: SpeechRecognitionErrorEvent) => {
        console.error('Speech recognition error:', event.error);
        setIsListening(false);

        let errorMessage = 'Speech recognition error';
        switch (event.error) {
          case 'no-speech':
            errorMessage = 'No speech detected. Please try again.';
            break;
          case 'audio-capture':
            errorMessage = 'Microphone not accessible. Please check permissions.';
            break;
          case 'not-allowed':
            errorMessage = 'Microphone permission denied. Please allow microphone access.';
            break;
          case 'network':
            errorMessage = 'Network error. Please check your connection.';
            break;
          default:
            errorMessage = `Error: ${event.error}`;
        }
        setError(errorMessage);
      };

      recognition.onend = () => {
        setIsListening(false);

        // If we have a final transcript, process it
        if (finalTranscriptRef.current.trim() && !isProcessing) {
          processCommand(finalTranscriptRef.current.trim());
        }
      };

      recognitionRef.current = recognition;
    }

    return () => {
      if (recognitionRef.current) {
        recognitionRef.current.stop();
      }
    };
  }, [isProcessing]);

  const startListening = () => {
    if (recognitionRef.current && !isListening && !isProcessing) {
      setTranscript('');
      setResult(null);
      setError(null);
      recognitionRef.current.start();
    }
  };

  const stopListening = () => {
    if (recognitionRef.current && isListening) {
      recognitionRef.current.stop();
    }
  };

  const processCommand = async (command: string) => {
    if (!command.trim() || isProcessing) return;

    setIsProcessing(true);
    setError(null);

    try {
      const response = await fetch('/api/v2/voice/process', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ command }),
      });

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({ error: 'Unknown error' }));
        throw new Error(errorData.error || `HTTP ${response.status}`);
      }

      const responseData = await response.json();

      // Handle ApiResponse wrapper
      const data = responseData.data || responseData;

      const result: VoiceCommandResult = {
        command,
        parsed: {
          apiType: data.parsed?.api_type || 'unknown',
          title: data.parsed?.title || 'Untitled API',
          description: data.parsed?.description || '',
          endpoints: data.parsed?.endpoints?.length || 0,
          models: data.parsed?.models?.length || 0,
        },
        spec: data.spec ? {
          title: data.spec.info?.title || data.spec.title || 'Generated API',
          version: data.spec.info?.version || data.spec.version || '1.0.0',
          json: JSON.stringify(data.spec, null, 2),
        } : undefined,
      };

      setResult(result);
      onCommandProcessed?.(result);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to process command';
      setError(errorMessage);
      setResult({
        command,
        parsed: {
          apiType: 'unknown',
          title: 'Error',
          description: '',
          endpoints: 0,
          models: 0,
        },
        error: errorMessage,
      });
    } finally {
      setIsProcessing(false);
    }
  };

  const handleTextSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (transcript.trim() && !isProcessing) {
      processCommand(transcript.trim());
    }
  };

  const downloadSpec = () => {
    if (!result?.spec) return;

    const blob = new Blob([result.spec.json], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${result.spec.title.replace(/\s+/g, '-').toLowerCase()}-${result.spec.version}.json`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  };

  if (!isSupported) {
    return (
      <div className={cn("p-6 border rounded-lg bg-muted/50", className)}>
        <div className="flex items-center gap-3 text-muted-foreground">
          <MicOff className="h-5 w-5" />
          <div>
            <p className="font-medium">Voice input not supported</p>
            <p className="text-sm">Your browser doesn't support the Web Speech API. Please use Chrome, Edge, or Safari.</p>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className={cn("space-y-4", className)}>
      {/* Voice Input Controls */}
      <div className="p-6 border rounded-lg bg-background">
        <div className="flex items-center gap-4">
          <Button
            onClick={isListening ? stopListening : startListening}
            disabled={isProcessing}
            variant={isListening ? "destructive" : "default"}
            size="lg"
            className="flex items-center gap-2"
          >
            {isListening ? (
              <>
                <MicOff className="h-5 w-5" />
                Stop Listening
              </>
            ) : (
              <>
                <Mic className="h-5 w-5" />
                Start Voice Input
              </>
            )}
          </Button>

          {isProcessing && (
            <div className="flex items-center gap-2 text-muted-foreground">
              <Loader2 className="h-4 w-4 animate-spin" />
              <span className="text-sm">Processing command...</span>
            </div>
          )}

          {isListening && (
            <div className="flex items-center gap-2 text-primary">
              <div className="h-2 w-2 bg-primary rounded-full animate-pulse" />
              <span className="text-sm font-medium">Listening...</span>
            </div>
          )}
        </div>

        {/* Transcript Display */}
        {(transcript || result) && (
          <div className="mt-4 p-4 bg-muted rounded-lg">
            <div className="text-sm font-medium text-muted-foreground mb-2">Command:</div>
            <div className="text-base">{result?.command || transcript}</div>
          </div>
        )}

        {/* Error Display */}
        {error && (
          <div className="mt-4 p-4 bg-destructive/10 border border-destructive/20 rounded-lg">
            <div className="flex items-center gap-2 text-destructive">
              <XCircle className="h-4 w-4" />
              <span className="text-sm font-medium">{error}</span>
            </div>
          </div>
        )}

        {/* Text Input Fallback */}
        <form onSubmit={handleTextSubmit} className="mt-4">
          <div className="flex gap-2">
            <input
              type="text"
              value={transcript}
              onChange={(e) => setTranscript(e.target.value)}
              placeholder="Or type your command here..."
              className="flex-1 px-4 py-2 border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary"
              disabled={isProcessing || isListening}
            />
            <Button type="submit" disabled={!transcript.trim() || isProcessing || isListening}>
              Process
            </Button>
          </div>
        </form>
      </div>

      {/* Results Display */}
      {result && (
        <div className="p-6 border rounded-lg bg-background space-y-4">
          {result.error ? (
            <div className="p-4 bg-destructive/10 border border-destructive/20 rounded-lg">
              <div className="flex items-center gap-2 text-destructive">
                <XCircle className="h-5 w-5" />
                <span className="font-medium">Error processing command</span>
              </div>
              <p className="mt-2 text-sm text-destructive/80">{result.error}</p>
            </div>
          ) : (
            <>
              <div className="flex items-center gap-2 text-green-600 dark:text-green-400">
                <CheckCircle2 className="h-5 w-5" />
                <span className="font-medium">Command processed successfully</span>
              </div>

              <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
                <div className="p-3 bg-muted rounded-lg">
                  <div className="text-xs text-muted-foreground mb-1">API Type</div>
                  <div className="font-medium">{result.parsed.apiType}</div>
                </div>
                <div className="p-3 bg-muted rounded-lg">
                  <div className="text-xs text-muted-foreground mb-1">Endpoints</div>
                  <div className="font-medium">{result.parsed.endpoints}</div>
                </div>
                <div className="p-3 bg-muted rounded-lg">
                  <div className="text-xs text-muted-foreground mb-1">Models</div>
                  <div className="font-medium">{result.parsed.models}</div>
                </div>
                <div className="p-3 bg-muted rounded-lg">
                  <div className="text-xs text-muted-foreground mb-1">Title</div>
                  <div className="font-medium truncate">{result.parsed.title}</div>
                </div>
              </div>

              {result.spec && (
                <div className="mt-4 space-y-2">
                  <div className="flex items-center justify-between">
                    <div>
                      <div className="font-medium">{result.spec.title}</div>
                      <div className="text-sm text-muted-foreground">Version {result.spec.version}</div>
                    </div>
                    <div className="flex gap-2">
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={downloadSpec}
                        className="flex items-center gap-2"
                      >
                        <Download className="h-4 w-4" />
                        Download Spec
                      </Button>
                    </div>
                  </div>
                </div>
              )}
            </>
          )}
        </div>
      )}
    </div>
  );
}

// TypeScript declarations for Web Speech API
interface SpeechRecognition extends EventTarget {
  continuous: boolean;
  interimResults: boolean;
  lang: string;
  start(): void;
  stop(): void;
  abort(): void;
  onstart: ((this: SpeechRecognition, ev: Event) => any) | null;
  onresult: ((this: SpeechRecognition, ev: SpeechRecognitionEvent) => any) | null;
  onerror: ((this: SpeechRecognition, ev: SpeechRecognitionErrorEvent) => any) | null;
  onend: ((this: SpeechRecognition, ev: Event) => any) | null;
}

interface SpeechRecognitionEvent extends Event {
  resultIndex: number;
  results: SpeechRecognitionResultList;
}

interface SpeechRecognitionErrorEvent extends Event {
  error: string;
}

interface SpeechRecognitionResultList {
  length: number;
  item(index: number): SpeechRecognitionResult;
  [index: number]: SpeechRecognitionResult;
}

interface SpeechRecognitionResult {
  length: number;
  item(index: number): SpeechRecognitionAlternative;
  [index: number]: SpeechRecognitionAlternative;
  isFinal: boolean;
}

interface SpeechRecognitionAlternative {
  transcript: string;
  confidence: number;
}

declare var SpeechRecognition: {
  prototype: SpeechRecognition;
  new (): SpeechRecognition;
};

declare let webkitSpeechRecognition: {
  prototype: SpeechRecognition;
  new (): SpeechRecognition;
};
