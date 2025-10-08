import React, { useState, useEffect } from 'react';
import {
  Box,
  Container,
  Typography,
  Paper,
  Grid,
  Card,
  CardContent,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Chip,
  LinearProgress,
  IconButton,
  Tooltip,
  Button,
  TextField,
  InputAdornment,
} from '@mui/material';
import {
  CheckCircle as CheckCircleIcon,
  Error as ErrorIcon,
  Timer as TimerIcon,
  Assessment as AssessmentIcon,
  Refresh as RefreshIcon,
  Search as SearchIcon,
  PlayArrow as PlayArrowIcon,
  Stop as StopIcon,
} from '@mui/icons-material';
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip as ChartTooltip,
  Legend,
  ResponsiveContainer,
  PieChart,
  Pie,
  Cell,
  LineChart,
  Line,
} from 'recharts';

interface TestExecution {
  id: string;
  workflow_id: string;
  workflow_name: string;
  status: 'running' | 'completed' | 'failed';
  started_at: string;
  completed_at?: string;
  duration_ms?: number;
  total_steps: number;
  completed_steps: number;
  failed_steps: number;
  success_rate: number;
}

interface TestMetrics {
  total_executions: number;
  successful_executions: number;
  failed_executions: number;
  average_duration_ms: number;
  executions_by_day: Array<{ date: string; count: number }>;
  executions_by_status: Array<{ name: string; value: number }>;
}

const TestExecutionDashboard: React.FC = () => {
  const [executions, setExecutions] = useState<TestExecution[]>([]);
  const [metrics, setMetrics] = useState<TestMetrics | null>(null);
  const [loading, setLoading] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');

  // Mock data for demonstration
  useEffect(() => {
    loadMockData();
  }, []);

  const loadMockData = () => {
    const mockExecutions: TestExecution[] = [
      {
        id: '1',
        workflow_id: 'wf-1',
        workflow_name: 'User Registration Flow',
        status: 'completed',
        started_at: new Date(Date.now() - 3600000).toISOString(),
        completed_at: new Date(Date.now() - 3500000).toISOString(),
        duration_ms: 100000,
        total_steps: 5,
        completed_steps: 5,
        failed_steps: 0,
        success_rate: 100,
      },
      {
        id: '2',
        workflow_id: 'wf-2',
        workflow_name: 'E-commerce Checkout',
        status: 'failed',
        started_at: new Date(Date.now() - 7200000).toISOString(),
        completed_at: new Date(Date.now() - 7100000).toISOString(),
        duration_ms: 100000,
        total_steps: 8,
        completed_steps: 6,
        failed_steps: 2,
        success_rate: 75,
      },
      {
        id: '3',
        workflow_id: 'wf-1',
        workflow_name: 'User Registration Flow',
        status: 'running',
        started_at: new Date(Date.now() - 60000).toISOString(),
        total_steps: 5,
        completed_steps: 3,
        failed_steps: 0,
        success_rate: 60,
      },
    ];

    const mockMetrics: TestMetrics = {
      total_executions: 156,
      successful_executions: 142,
      failed_executions: 14,
      average_duration_ms: 85000,
      executions_by_day: [
        { date: '2025-10-01', count: 12 },
        { date: '2025-10-02', count: 18 },
        { date: '2025-10-03', count: 25 },
        { date: '2025-10-04', count: 21 },
        { date: '2025-10-05', count: 19 },
        { date: '2025-10-06', count: 24 },
        { date: '2025-10-07', count: 22 },
        { date: '2025-10-08', count: 15 },
      ],
      executions_by_status: [
        { name: 'Success', value: 142 },
        { name: 'Failed', value: 14 },
      ],
    };

    setExecutions(mockExecutions);
    setMetrics(mockMetrics);
  };

  const handleRefresh = () => {
    setLoading(true);
    setTimeout(() => {
      loadMockData();
      setLoading(false);
    }, 1000);
  };

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'completed':
        return <CheckCircleIcon color="success" />;
      case 'failed':
        return <ErrorIcon color="error" />;
      case 'running':
        return <TimerIcon color="info" />;
      default:
        return <TimerIcon />;
    }
  };

  const getStatusColor = (status: string): 'success' | 'error' | 'info' | 'default' => {
    switch (status) {
      case 'completed':
        return 'success';
      case 'failed':
        return 'error';
      case 'running':
        return 'info';
      default:
        return 'default';
    }
  };

  const formatDuration = (ms?: number) => {
    if (!ms) return 'N/A';
    const seconds = Math.floor(ms / 1000);
    if (seconds < 60) return `${seconds}s`;
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = seconds % 60;
    return `${minutes}m ${remainingSeconds}s`;
  };

  const filteredExecutions = executions.filter((exec) =>
    exec.workflow_name.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const COLORS = ['#4caf50', '#f44336', '#2196f3'];

  return (
    <Container maxWidth="xl">
      <Box sx={{ my: 4 }}>
        <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 3 }}>
          <Typography variant="h4">
            <AssessmentIcon sx={{ mr: 1, verticalAlign: 'middle' }} />
            Test Execution Dashboard
          </Typography>
          <Button startIcon={<RefreshIcon />} onClick={handleRefresh} disabled={loading}>
            Refresh
          </Button>
        </Box>

        {loading && <LinearProgress sx={{ mb: 2 }} />}

        {/* Metrics Overview */}
        {metrics && (
          <Grid container spacing={3} sx={{ mb: 3 }}>
            <Grid item xs={12} md={3}>
              <Card>
                <CardContent>
                  <Typography color="text.secondary" gutterBottom>
                    Total Executions
                  </Typography>
                  <Typography variant="h4">{metrics.total_executions}</Typography>
                </CardContent>
              </Card>
            </Grid>

            <Grid item xs={12} md={3}>
              <Card>
                <CardContent>
                  <Typography color="text.secondary" gutterBottom>
                    Success Rate
                  </Typography>
                  <Typography variant="h4" color="success.main">
                    {((metrics.successful_executions / metrics.total_executions) * 100).toFixed(1)}%
                  </Typography>
                </CardContent>
              </Card>
            </Grid>

            <Grid item xs={12} md={3}>
              <Card>
                <CardContent>
                  <Typography color="text.secondary" gutterBottom>
                    Failed Tests
                  </Typography>
                  <Typography variant="h4" color="error.main">
                    {metrics.failed_executions}
                  </Typography>
                </CardContent>
              </Card>
            </Grid>

            <Grid item xs={12} md={3}>
              <Card>
                <CardContent>
                  <Typography color="text.secondary" gutterBottom>
                    Avg Duration
                  </Typography>
                  <Typography variant="h4">{formatDuration(metrics.average_duration_ms)}</Typography>
                </CardContent>
              </Card>
            </Grid>
          </Grid>
        )}

        {/* Charts */}
        {metrics && (
          <Grid container spacing={3} sx={{ mb: 3 }}>
            <Grid item xs={12} md={8}>
              <Paper sx={{ p: 3 }}>
                <Typography variant="h6" gutterBottom>
                  Executions Over Time
                </Typography>
                <ResponsiveContainer width="100%" height={300}>
                  <LineChart data={metrics.executions_by_day}>
                    <CartesianGrid strokeDasharray="3 3" />
                    <XAxis dataKey="date" />
                    <YAxis />
                    <ChartTooltip />
                    <Legend />
                    <Line type="monotone" dataKey="count" stroke="#2196f3" name="Executions" />
                  </LineChart>
                </ResponsiveContainer>
              </Paper>
            </Grid>

            <Grid item xs={12} md={4}>
              <Paper sx={{ p: 3 }}>
                <Typography variant="h6" gutterBottom>
                  Status Distribution
                </Typography>
                <ResponsiveContainer width="100%" height={300}>
                  <PieChart>
                    <Pie
                      data={metrics.executions_by_status}
                      cx="50%"
                      cy="50%"
                      labelLine={false}
                      label={(entry) => `${entry.name}: ${entry.value}`}
                      outerRadius={80}
                      fill="#8884d8"
                      dataKey="value"
                    >
                      {metrics.executions_by_status.map((entry, index) => (
                        <Cell key={`cell-${index}`} fill={COLORS[index % COLORS.length]} />
                      ))}
                    </Pie>
                    <ChartTooltip />
                  </PieChart>
                </ResponsiveContainer>
              </Paper>
            </Grid>
          </Grid>
        )}

        {/* Execution List */}
        <Paper>
          <Box sx={{ p: 2, borderBottom: 1, borderColor: 'divider' }}>
            <TextField
              size="small"
              placeholder="Search workflows..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              InputProps={{
                startAdornment: (
                  <InputAdornment position="start">
                    <SearchIcon />
                  </InputAdornment>
                ),
              }}
              sx={{ width: 300 }}
            />
          </Box>

          <TableContainer>
            <Table>
              <TableHead>
                <TableRow>
                  <TableCell>Status</TableCell>
                  <TableCell>Workflow</TableCell>
                  <TableCell>Started</TableCell>
                  <TableCell>Duration</TableCell>
                  <TableCell>Progress</TableCell>
                  <TableCell>Success Rate</TableCell>
                  <TableCell>Actions</TableCell>
                </TableRow>
              </TableHead>
              <TableBody>
                {filteredExecutions.map((execution) => (
                  <TableRow key={execution.id}>
                    <TableCell>
                      <Chip
                        icon={getStatusIcon(execution.status)}
                        label={execution.status.toUpperCase()}
                        color={getStatusColor(execution.status)}
                        size="small"
                      />
                    </TableCell>
                    <TableCell>
                      <Typography variant="body2">{execution.workflow_name}</Typography>
                      <Typography variant="caption" color="text.secondary">
                        ID: {execution.workflow_id}
                      </Typography>
                    </TableCell>
                    <TableCell>
                      <Typography variant="body2">
                        {new Date(execution.started_at).toLocaleString()}
                      </Typography>
                    </TableCell>
                    <TableCell>{formatDuration(execution.duration_ms)}</TableCell>
                    <TableCell>
                      <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                        <LinearProgress
                          variant="determinate"
                          value={(execution.completed_steps / execution.total_steps) * 100}
                          sx={{ flexGrow: 1, height: 8, borderRadius: 4 }}
                        />
                        <Typography variant="caption">
                          {execution.completed_steps}/{execution.total_steps}
                        </Typography>
                      </Box>
                    </TableCell>
                    <TableCell>
                      <Typography variant="body2" color={execution.success_rate === 100 ? 'success.main' : execution.success_rate < 50 ? 'error.main' : 'warning.main'}>
                        {execution.success_rate}%
                      </Typography>
                    </TableCell>
                    <TableCell>
                      {execution.status === 'running' ? (
                        <Tooltip title="Stop">
                          <IconButton size="small" color="error">
                            <StopIcon />
                          </IconButton>
                        </Tooltip>
                      ) : (
                        <Tooltip title="Re-run">
                          <IconButton size="small" color="primary">
                            <PlayArrowIcon />
                          </IconButton>
                        </Tooltip>
                      )}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </TableContainer>
        </Paper>
      </Box>
    </Container>
  );
};

export default TestExecutionDashboard;
