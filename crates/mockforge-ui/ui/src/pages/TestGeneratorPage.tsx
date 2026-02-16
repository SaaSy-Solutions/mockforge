import React, { useState } from 'react';
import {
  Box,
  Container,
  Typography,
  Paper,
  Grid,
  Button,
  TextField,
  Select,
  MenuItem,
  FormControl,
  InputLabel,
  Switch,
  FormControlLabel,
  Chip,
  Alert,
  Accordion,
  AccordionSummary,
  AccordionDetails,
  LinearProgress,
  Card,
  CardContent,
  IconButton,
  Tooltip,
} from '@mui/material';
import {
  ExpandMore as ExpandMoreIcon,
  Download as DownloadIcon,
  PlayArrow as PlayArrowIcon,
  Code as CodeIcon,
  BugReport as BugReportIcon,
  Assessment as AssessmentIcon,
  AutoFixHigh as AutoFixHighIcon,
} from '@mui/icons-material';

interface TestFixture {
  name: string;
  description: string;
  data: any;
  endpoints: string[];
}

interface EdgeCase {
  endpoint: string;
  method: string;
  case_type: string;
  description: string;
  expected_behavior: string;
  priority: number;
}

interface TestGapAnalysis {
  untested_endpoints: string[];
  missing_methods: { [key: string]: string[] };
  missing_status_codes: { [key: string]: number[] };
  missing_error_scenarios: string[];
  coverage_percentage: number;
  recommendations: string[];
}

function CodeBlock({ content, maxHeight = '500px' }: { content: string; maxHeight?: string }) {
  return (
    <Box
      component="pre"
      sx={{
        maxHeight,
        overflow: 'auto',
        p: 2,
        m: 0,
        borderRadius: 1,
        backgroundColor: '#0f172a',
        color: '#e2e8f0',
        fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace',
        fontSize: '0.8rem',
        lineHeight: 1.5,
      }}
    >
      <Box component="code">{content}</Box>
    </Box>
  );
}

const TestGeneratorPage: React.FC = () => {
  const [format, setFormat] = useState('rust_reqwest');
  const [protocol, setProtocol] = useState('Http');
  const [limit, setLimit] = useState(50);
  const [aiDescriptions, setAiDescriptions] = useState(false);
  const [generateFixtures, setGenerateFixtures] = useState(false);
  const [suggestEdgeCases, setSuggestEdgeCases] = useState(false);
  const [analyzeGaps, setAnalyzeGaps] = useState(false);
  const [deduplicateTests, setDeduplicateTests] = useState(true);
  const [optimizeOrder, setOptimizeOrder] = useState(true);

  const [loading, setLoading] = useState(false);
  const [generatedTests, setGeneratedTests] = useState<string>('');
  const [metadata, setMetadata] = useState<any>(null);
  const [fixtures, setFixtures] = useState<TestFixture[]>([]);
  const [edgeCases, setEdgeCases] = useState<EdgeCase[]>([]);
  const [gapAnalysis, setGapAnalysis] = useState<TestGapAnalysis | null>(null);

  const testFormats = [
    { value: 'rust_reqwest', label: 'Rust (reqwest)' },
    { value: 'python_pytest', label: 'Python (pytest)' },
    { value: 'javascript_jest', label: 'JavaScript (Jest)' },
    { value: 'go_test', label: 'Go (testing)' },
    { value: 'ruby_rspec', label: 'Ruby (RSpec)' },
    { value: 'java_junit', label: 'Java (JUnit)' },
    { value: 'csharp_xunit', label: 'C# (xUnit)' },
    { value: 'http_file', label: 'HTTP File (.http)' },
    { value: 'curl', label: 'cURL' },
    { value: 'postman', label: 'Postman Collection' },
    { value: 'k6', label: 'k6 Load Test' },
  ];

  const handleGenerate = async () => {
    setLoading(true);
    try {
      const response = await fetch('/api/recorder/generate-tests', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          format,
          filter: {
            protocol,
            limit,
          },
          ai_descriptions: aiDescriptions,
          generate_fixtures: generateFixtures,
          suggest_edge_cases: suggestEdgeCases,
          analyze_test_gaps: analyzeGaps,
          deduplicate_tests: deduplicateTests,
          optimize_test_order: optimizeOrder,
          llm_config: aiDescriptions ? {
            provider: 'ollama',
            api_endpoint: 'http://localhost:11434/api/generate',
            model: 'llama2',
            temperature: 0.3,
          } : null,
        }),
      });

      const data = await response.json();

      if (data.success) {
        setGeneratedTests(data.test_file);
        setMetadata(data.metadata);

        if (data.metadata.fixtures) {
          setFixtures(data.metadata.fixtures);
        }
        if (data.metadata.edge_cases) {
          setEdgeCases(data.metadata.edge_cases);
        }
        if (data.metadata.gap_analysis) {
          setGapAnalysis(data.metadata.gap_analysis);
        }
      }
    } catch (error) {
      console.error('Failed to generate tests:', error);
    } finally {
      setLoading(false);
    }
  };

  const handleDownload = () => {
    const blob = new Blob([generatedTests], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `generated_tests.${getFileExtension(format)}`;
    a.click();
    URL.revokeObjectURL(url);
  };

  const getFileExtension = (format: string): string => {
    const extensions: { [key: string]: string } = {
      rust_reqwest: 'rs',
      python_pytest: 'py',
      javascript_jest: 'js',
      go_test: 'go',
      ruby_rspec: 'rb',
      java_junit: 'java',
      csharp_xunit: 'cs',
      http_file: 'http',
      curl: 'sh',
      postman: 'json',
      k6: 'js',
    };
    return extensions[format] || 'txt';
  };

  return (
    <Container maxWidth="xl">
      <Box sx={{ my: 4 }}>
        <Typography variant="h4" gutterBottom>
          <CodeIcon sx={{ mr: 1, verticalAlign: 'middle' }} />
          Test Generator
        </Typography>
        <Typography variant="body1" color="text.secondary" paragraph>
          Generate test cases from recorded API interactions with AI-powered insights
        </Typography>

        <Grid container spacing={3}>
          {/* Configuration Panel */}
          <Grid item xs={12} md={4}>
            <Paper sx={{ p: 3 }}>
              <Typography variant="h6" gutterBottom>
                Configuration
              </Typography>

              <FormControl fullWidth sx={{ mb: 2 }}>
                <InputLabel>Test Format</InputLabel>
                <Select value={format} onChange={(e) => setFormat(e.target.value)} label="Test Format">
                  {testFormats.map((f) => (
                    <MenuItem key={f.value} value={f.value}>
                      {f.label}
                    </MenuItem>
                  ))}
                </Select>
              </FormControl>

              <FormControl fullWidth sx={{ mb: 2 }}>
                <InputLabel>Protocol</InputLabel>
                <Select value={protocol} onChange={(e) => setProtocol(e.target.value)} label="Protocol">
                  <MenuItem value="Http">HTTP</MenuItem>
                  <MenuItem value="Grpc">gRPC</MenuItem>
                  <MenuItem value="GraphQL">GraphQL</MenuItem>
                  <MenuItem value="WebSocket">WebSocket</MenuItem>
                </Select>
              </FormControl>

              <TextField
                fullWidth
                label="Max Tests"
                type="number"
                value={limit}
                onChange={(e) => setLimit(parseInt(e.target.value))}
                sx={{ mb: 3 }}
              />

              <Typography variant="subtitle2" gutterBottom sx={{ mt: 2 }}>
                AI Features
              </Typography>

              <FormControlLabel
                control={<Switch checked={aiDescriptions} onChange={(e) => setAiDescriptions(e.target.checked)} />}
                label="AI Descriptions"
              />

              <FormControlLabel
                control={<Switch checked={generateFixtures} onChange={(e) => setGenerateFixtures(e.target.checked)} />}
                label="Generate Fixtures"
              />

              <FormControlLabel
                control={<Switch checked={suggestEdgeCases} onChange={(e) => setSuggestEdgeCases(e.target.checked)} />}
                label="Suggest Edge Cases"
              />

              <FormControlLabel
                control={<Switch checked={analyzeGaps} onChange={(e) => setAnalyzeGaps(e.target.checked)} />}
                label="Analyze Test Gaps"
              />

              <Typography variant="subtitle2" gutterBottom sx={{ mt: 2 }}>
                Optimization
              </Typography>

              <FormControlLabel
                control={<Switch checked={deduplicateTests} onChange={(e) => setDeduplicateTests(e.target.checked)} />}
                label="Deduplicate Tests"
              />

              <FormControlLabel
                control={<Switch checked={optimizeOrder} onChange={(e) => setOptimizeOrder(e.target.checked)} />}
                label="Optimize Order"
              />

              <Button
                fullWidth
                variant="contained"
                size="large"
                onClick={handleGenerate}
                disabled={loading}
                startIcon={<PlayArrowIcon />}
                sx={{ mt: 3 }}
              >
                Generate Tests
              </Button>

              {loading && <LinearProgress sx={{ mt: 2 }} />}
            </Paper>

            {/* Metadata Card */}
            {metadata && (
              <Card sx={{ mt: 2 }}>
                <CardContent>
                  <Typography variant="h6" gutterBottom>
                    Metadata
                  </Typography>
                  <Typography variant="body2">
                    Tests Generated: <strong>{metadata.test_count}</strong>
                  </Typography>
                  <Typography variant="body2">
                    Endpoints Covered: <strong>{metadata.endpoint_count}</strong>
                  </Typography>
                  {gapAnalysis && (
                    <Typography variant="body2" color="primary">
                      Coverage: <strong>{gapAnalysis.coverage_percentage.toFixed(1)}%</strong>
                    </Typography>
                  )}
                </CardContent>
              </Card>
            )}
          </Grid>

          {/* Results Panel */}
          <Grid item xs={12} md={8}>
            {generatedTests && (
              <Paper sx={{ p: 3 }}>
                <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 2 }}>
                  <Typography variant="h6">
                    Generated Tests
                  </Typography>
                  <Button
                    variant="outlined"
                    startIcon={<DownloadIcon />}
                    onClick={handleDownload}
                  >
                    Download
                  </Button>
                </Box>

                <CodeBlock content={generatedTests} maxHeight="500px" />

                {/* Fixtures */}
                {fixtures.length > 0 && (
                  <Accordion sx={{ mt: 2 }}>
                    <AccordionSummary expandIcon={<ExpandMoreIcon />}>
                      <Typography>
                        <AutoFixHighIcon sx={{ mr: 1, verticalAlign: 'middle' }} />
                        Test Fixtures ({fixtures.length})
                      </Typography>
                    </AccordionSummary>
                    <AccordionDetails>
                      {fixtures.map((fixture, idx) => (
                        <Box key={idx} sx={{ mb: 2 }}>
                          <Typography variant="subtitle2">{fixture.name}</Typography>
                          <Typography variant="body2" color="text.secondary">{fixture.description}</Typography>
                          <CodeBlock content={JSON.stringify(fixture.data, null, 2)} maxHeight="200px" />
                        </Box>
                      ))}
                    </AccordionDetails>
                  </Accordion>
                )}

                {/* Edge Cases */}
                {edgeCases.length > 0 && (
                  <Accordion sx={{ mt: 2 }}>
                    <AccordionSummary expandIcon={<ExpandMoreIcon />}>
                      <Typography>
                        <BugReportIcon sx={{ mr: 1, verticalAlign: 'middle' }} />
                        Edge Case Suggestions ({edgeCases.length})
                      </Typography>
                    </AccordionSummary>
                    <AccordionDetails>
                      {edgeCases.map((edge, idx) => (
                        <Alert
                          key={idx}
                          severity={edge.priority >= 4 ? 'error' : edge.priority >= 3 ? 'warning' : 'info'}
                          sx={{ mb: 1 }}
                        >
                          <Typography variant="subtitle2">
                            {edge.method} {edge.endpoint} - {edge.case_type}
                          </Typography>
                          <Typography variant="body2">{edge.description}</Typography>
                          <Typography variant="caption" display="block">
                            Expected: {edge.expected_behavior}
                          </Typography>
                          <Chip label={`Priority: ${edge.priority}`} size="small" sx={{ mt: 1 }} />
                        </Alert>
                      ))}
                    </AccordionDetails>
                  </Accordion>
                )}

                {/* Gap Analysis */}
                {gapAnalysis && (
                  <Accordion sx={{ mt: 2 }}>
                    <AccordionSummary expandIcon={<ExpandMoreIcon />}>
                      <Typography>
                        <AssessmentIcon sx={{ mr: 1, verticalAlign: 'middle' }} />
                        Test Gap Analysis ({gapAnalysis.coverage_percentage.toFixed(1)}% Coverage)
                      </Typography>
                    </AccordionSummary>
                    <AccordionDetails>
                      {gapAnalysis.untested_endpoints.length > 0 && (
                        <Box sx={{ mb: 2 }}>
                          <Typography variant="subtitle2">Untested Endpoints:</Typography>
                          {gapAnalysis.untested_endpoints.map((endpoint, idx) => (
                            <Chip key={idx} label={endpoint} size="small" sx={{ m: 0.5 }} />
                          ))}
                        </Box>
                      )}

                      {Object.keys(gapAnalysis.missing_methods).length > 0 && (
                        <Box sx={{ mb: 2 }}>
                          <Typography variant="subtitle2">Missing Methods:</Typography>
                          {Object.entries(gapAnalysis.missing_methods).map(([endpoint, methods], idx) => (
                            <Typography key={idx} variant="body2">
                              {endpoint}: {methods.join(', ')}
                            </Typography>
                          ))}
                        </Box>
                      )}

                      {gapAnalysis.recommendations.length > 0 && (
                        <Box>
                          <Typography variant="subtitle2">Recommendations:</Typography>
                          {gapAnalysis.recommendations.map((rec, idx) => (
                            <Alert key={idx} severity="info" sx={{ mt: 1 }}>
                              {rec}
                            </Alert>
                          ))}
                        </Box>
                      )}
                    </AccordionDetails>
                  </Accordion>
                )}
              </Paper>
            )}

            {!generatedTests && !loading && (
              <Paper sx={{ p: 6, textAlign: 'center' }}>
                <CodeIcon sx={{ fontSize: 64, color: 'text.disabled', mb: 2 }} />
                <Typography variant="h6" color="text.secondary">
                  Configure and generate tests
                </Typography>
                <Typography variant="body2" color="text.secondary">
                  Select your preferences and click "Generate Tests" to get started
                </Typography>
              </Paper>
            )}
          </Grid>
        </Grid>
      </Box>
    </Container>
  );
};

export default TestGeneratorPage;
