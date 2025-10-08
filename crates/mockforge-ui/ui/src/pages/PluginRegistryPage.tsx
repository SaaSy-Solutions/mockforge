/**
 * Plugin Registry Page
 *
 * Browse, search, install, and manage plugins from the MockForge registry.
 */

import React, { useState, useEffect } from 'react';
import {
  Box,
  Card,
  CardContent,
  CardActions,
  Grid,
  Typography,
  TextField,
  Button,
  Chip,
  Rating,
  IconButton,
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  FormControl,
  InputLabel,
  Select,
  MenuItem,
  Tabs,
  Tab,
  List,
  ListItem,
  ListItemText,
  Divider,
  Alert,
  LinearProgress,
  Tooltip,
  Badge,
  Avatar,
  Paper,
  Stack,
} from '@mui/material';
import {
  Search as SearchIcon,
  Star as StarIcon,
  Download as DownloadIcon,
  Visibility as ViewIcon,
  Category as CategoryIcon,
  Security as SecurityIcon,
  Code as CodeIcon,
  GitHub as GitHubIcon,
  Language as LanguageIcon,
  Update as UpdateIcon,
  CheckCircle as CheckCircleIcon,
  Warning as WarningIcon,
  Error as ErrorIcon,
  ThumbUp as ThumbUpIcon,
  ThumbDown as ThumbDownIcon,
} from '@mui/icons-material';

interface Plugin {
  name: string;
  description: string;
  version: string;
  author: AuthorInfo;
  category: string;
  language: string;
  tags: string[];
  license: string;
  repository?: string;
  homepage?: string;
  downloads: number;
  rating: number;
  reviewsCount: number;
  securityScore: number;
  createdAt: string;
  updatedAt: string;
  versions: VersionInfo[];
}

interface AuthorInfo {
  name: string;
  email?: string;
  url?: string;
}

interface VersionInfo {
  version: string;
  publishedAt: string;
  yanked: boolean;
  downloadUrl: string;
  checksum: string;
}

interface Review {
  id: string;
  userName: string;
  userAvatar?: string;
  rating: number;
  title?: string;
  comment: string;
  createdAt: string;
  helpfulCount: number;
  unhelpfulCount: number;
  verified: boolean;
  authorResponse?: {
    text: string;
    createdAt: string;
  };
}

interface SecurityScanResult {
  status: 'pass' | 'warning' | 'fail';
  score: number;
  findings: SecurityFinding[];
}

interface SecurityFinding {
  severity: 'info' | 'low' | 'medium' | 'high' | 'critical';
  category: string;
  title: string;
  description: string;
}

export const PluginRegistryPage: React.FC = () => {
  const [plugins, setPlugins] = useState<Plugin[]>([]);
  const [filteredPlugins, setFilteredPlugins] = useState<Plugin[]>([]);
  const [selectedPlugin, setSelectedPlugin] = useState<Plugin | null>(null);
  const [reviews, setReviews] = useState<Review[]>([]);
  const [securityScan, setSecurityScan] = useState<SecurityScanResult | null>(null);
  const [detailsOpen, setDetailsOpen] = useState(false);
  const [reviewDialogOpen, setReviewDialogOpen] = useState(false);
  const [activeTab, setActiveTab] = useState(0);

  // Filters
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedCategory, setSelectedCategory] = useState('all');
  const [selectedLanguage, setSelectedLanguage] = useState('all');
  const [sortBy, setSortBy] = useState('popular');
  const [minRating, setMinRating] = useState(0);
  const [minSecurityScore, setMinSecurityScore] = useState(0);

  // Installation state
  const [installing, setInstalling] = useState<string | null>(null);
  const [installProgress, setInstallProgress] = useState(0);

  const categories = [
    { value: 'all', label: 'All Categories' },
    { value: 'auth', label: 'Authentication' },
    { value: 'template', label: 'Templates' },
    { value: 'response', label: 'Response' },
    { value: 'datasource', label: 'Data Source' },
    { value: 'middleware', label: 'Middleware' },
    { value: 'testing', label: 'Testing' },
    { value: 'observability', label: 'Observability' },
  ];

  const languages = [
    { value: 'all', label: 'All Languages' },
    { value: 'rust', label: 'Rust' },
    { value: 'python', label: 'Python' },
    { value: 'javascript', label: 'JavaScript' },
    { value: 'typescript', label: 'TypeScript' },
    { value: 'go', label: 'Go' },
  ];

  const sortOptions = [
    { value: 'popular', label: 'Most Popular' },
    { value: 'downloads', label: 'Most Downloaded' },
    { value: 'rating', label: 'Top Rated' },
    { value: 'recent', label: 'Recently Updated' },
    { value: 'security', label: 'Best Security Score' },
  ];

  useEffect(() => {
    loadPlugins();
  }, []);

  useEffect(() => {
    filterPlugins();
  }, [plugins, searchQuery, selectedCategory, selectedLanguage, sortBy, minRating, minSecurityScore]);

  const loadPlugins = async () => {
    try {
      const response = await fetch('/api/v1/plugins/search', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          query: null,
          category: null,
          tags: [],
          sort: sortBy,
          page: 0,
          per_page: 100,
        }),
      });

      if (response.ok) {
        const data = await response.json();
        setPlugins(data.plugins || []);
      }
    } catch (error) {
      console.error('Failed to load plugins:', error);
    }
  };

  const filterPlugins = () => {
    let filtered = [...plugins];

    // Search filter
    if (searchQuery) {
      const query = searchQuery.toLowerCase();
      filtered = filtered.filter(
        (p) =>
          p.name.toLowerCase().includes(query) ||
          p.description.toLowerCase().includes(query) ||
          p.author.name.toLowerCase().includes(query) ||
          p.tags.some((tag) => tag.toLowerCase().includes(query))
      );
    }

    // Category filter
    if (selectedCategory !== 'all') {
      filtered = filtered.filter((p) => p.category === selectedCategory);
    }

    // Language filter
    if (selectedLanguage !== 'all') {
      filtered = filtered.filter((p) => p.language === selectedLanguage);
    }

    // Rating filter
    if (minRating > 0) {
      filtered = filtered.filter((p) => p.rating >= minRating);
    }

    // Security score filter
    if (minSecurityScore > 0) {
      filtered = filtered.filter((p) => p.securityScore >= minSecurityScore);
    }

    // Sort
    switch (sortBy) {
      case 'popular':
        filtered.sort((a, b) => {
          const scoreA = a.downloads + a.rating * 100;
          const scoreB = b.downloads + b.rating * 100;
          return scoreB - scoreA;
        });
        break;
      case 'downloads':
        filtered.sort((a, b) => b.downloads - a.downloads);
        break;
      case 'rating':
        filtered.sort((a, b) => b.rating - a.rating);
        break;
      case 'recent':
        filtered.sort(
          (a, b) => new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime()
        );
        break;
      case 'security':
        filtered.sort((a, b) => b.securityScore - a.securityScore);
        break;
    }

    setFilteredPlugins(filtered);
  };

  const handleViewDetails = async (plugin: Plugin) => {
    setSelectedPlugin(plugin);
    setDetailsOpen(true);
    setActiveTab(0);

    // Load reviews
    try {
      const response = await fetch(`/api/v1/plugins/${plugin.name}/reviews`);
      if (response.ok) {
        const data = await response.json();
        setReviews(data.reviews || []);
      }
    } catch (error) {
      console.error('Failed to load reviews:', error);
    }

    // Load security scan
    try {
      const response = await fetch(`/api/v1/plugins/${plugin.name}/security`);
      if (response.ok) {
        const data = await response.json();
        setSecurityScan(data);
      }
    } catch (error) {
      console.error('Failed to load security scan:', error);
    }
  };

  const handleInstall = async (pluginName: string, version?: string) => {
    setInstalling(pluginName);
    setInstallProgress(0);

    try {
      const spec = version ? `${pluginName}@${version}` : pluginName;

      // Simulate progress
      const progressInterval = setInterval(() => {
        setInstallProgress((prev) => Math.min(prev + 10, 90));
      }, 200);

      const response = await fetch('/api/plugins/install', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ plugin: spec }),
      });

      clearInterval(progressInterval);
      setInstallProgress(100);

      if (response.ok) {
        setTimeout(() => {
          setInstalling(null);
          setInstallProgress(0);
          alert(`Plugin ${pluginName} installed successfully!`);
        }, 500);
      } else {
        throw new Error('Installation failed');
      }
    } catch (error) {
      console.error('Failed to install plugin:', error);
      setInstalling(null);
      setInstallProgress(0);
      alert(`Failed to install ${pluginName}`);
    }
  };

  const handleVoteReview = async (reviewId: string, helpful: boolean) => {
    try {
      await fetch(`/api/v1/reviews/${reviewId}/vote`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ helpful }),
      });
      // Reload reviews
      if (selectedPlugin) {
        handleViewDetails(selectedPlugin);
      }
    } catch (error) {
      console.error('Failed to vote on review:', error);
    }
  };

  const getSecurityBadge = (score: number) => {
    if (score >= 90) return { color: 'success', label: 'Excellent', icon: <CheckCircleIcon /> };
    if (score >= 70) return { color: 'info', label: 'Good', icon: <CheckCircleIcon /> };
    if (score >= 50) return { color: 'warning', label: 'Fair', icon: <WarningIcon /> };
    return { color: 'error', label: 'Poor', icon: <ErrorIcon /> };
  };

  return (
    <Box sx={{ p: 3 }}>
      {/* Header */}
      <Box sx={{ mb: 4 }}>
        <Typography variant="h4" gutterBottom>
          Plugin Registry
        </Typography>
        <Typography variant="body1" color="text.secondary">
          Discover and install plugins from the MockForge ecosystem
        </Typography>
      </Box>

      {/* Filters */}
      <Card sx={{ mb: 3 }}>
        <CardContent>
          <Grid container spacing={2} alignItems="center">
            <Grid item xs={12} md={3}>
              <TextField
                fullWidth
                placeholder="Search plugins..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                InputProps={{
                  startAdornment: <SearchIcon sx={{ mr: 1, color: 'text.secondary' }} />,
                }}
              />
            </Grid>
            <Grid item xs={6} md={2}>
              <FormControl fullWidth size="small">
                <InputLabel>Category</InputLabel>
                <Select
                  value={selectedCategory}
                  label="Category"
                  onChange={(e) => setSelectedCategory(e.target.value)}
                >
                  {categories.map((cat) => (
                    <MenuItem key={cat.value} value={cat.value}>
                      {cat.label}
                    </MenuItem>
                  ))}
                </Select>
              </FormControl>
            </Grid>
            <Grid item xs={6} md={2}>
              <FormControl fullWidth size="small">
                <InputLabel>Language</InputLabel>
                <Select
                  value={selectedLanguage}
                  label="Language"
                  onChange={(e) => setSelectedLanguage(e.target.value)}
                >
                  {languages.map((lang) => (
                    <MenuItem key={lang.value} value={lang.value}>
                      {lang.label}
                    </MenuItem>
                  ))}
                </Select>
              </FormControl>
            </Grid>
            <Grid item xs={6} md={2}>
              <FormControl fullWidth size="small">
                <InputLabel>Sort By</InputLabel>
                <Select
                  value={sortBy}
                  label="Sort By"
                  onChange={(e) => setSortBy(e.target.value)}
                >
                  {sortOptions.map((opt) => (
                    <MenuItem key={opt.value} value={opt.value}>
                      {opt.label}
                    </MenuItem>
                  ))}
                </Select>
              </FormControl>
            </Grid>
            <Grid item xs={6} md={1.5}>
              <Box>
                <Typography variant="caption" display="block" gutterBottom>
                  Min Rating
                </Typography>
                <Rating
                  value={minRating}
                  onChange={(_, value) => setMinRating(value || 0)}
                  size="small"
                />
              </Box>
            </Grid>
            <Grid item xs={6} md={1.5}>
              <Box>
                <Typography variant="caption" display="block" gutterBottom>
                  Min Security
                </Typography>
                <TextField
                  type="number"
                  size="small"
                  value={minSecurityScore}
                  onChange={(e) => setMinSecurityScore(Number(e.target.value))}
                  inputProps={{ min: 0, max: 100 }}
                  sx={{ width: '80px' }}
                />
              </Box>
            </Grid>
          </Grid>
        </CardContent>
      </Card>

      {/* Results Summary */}
      <Box sx={{ mb: 2 }}>
        <Typography variant="body2" color="text.secondary">
          {filteredPlugins.length} plugin{filteredPlugins.length !== 1 ? 's' : ''} found
        </Typography>
      </Box>

      {/* Plugins Grid */}
      <Grid container spacing={3}>
        {filteredPlugins.map((plugin) => {
          const securityBadge = getSecurityBadge(plugin.securityScore);
          return (
            <Grid item xs={12} md={6} lg={4} key={plugin.name}>
              <Card sx={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
                <CardContent sx={{ flexGrow: 1 }}>
                  <Box sx={{ display: 'flex', justifyContent: 'space-between', mb: 1.5 }}>
                    <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                      <LanguageIcon fontSize="small" color="primary" />
                      <Typography variant="caption" color="text.secondary">
                        {plugin.language}
                      </Typography>
                    </Box>
                    <Tooltip title={`Security Score: ${plugin.securityScore}/100`}>
                      <Chip
                        icon={securityBadge.icon}
                        label={securityBadge.label}
                        size="small"
                        color={securityBadge.color as any}
                      />
                    </Tooltip>
                  </Box>

                  <Box sx={{ display: 'flex', justifyContent: 'space-between', mb: 2 }}>
                    <Typography variant="h6">{plugin.name}</Typography>
                    <Chip label={plugin.category} size="small" color="primary" variant="outlined" />
                  </Box>

                  <Typography variant="body2" color="text.secondary" sx={{ mb: 2, minHeight: '40px' }}>
                    {plugin.description}
                  </Typography>

                  <Box sx={{ mb: 2 }}>
                    {plugin.tags.slice(0, 4).map((tag) => (
                      <Chip
                        key={tag}
                        label={tag}
                        size="small"
                        sx={{ mr: 0.5, mb: 0.5 }}
                        variant="outlined"
                      />
                    ))}
                  </Box>

                  <Box sx={{ display: 'flex', alignItems: 'center', gap: 2, mb: 1.5 }}>
                    <Rating value={plugin.rating} readOnly size="small" precision={0.5} />
                    <Typography variant="caption" color="text.secondary">
                      ({plugin.reviewsCount})
                    </Typography>
                  </Box>

                  <Grid container spacing={1}>
                    <Grid item xs={6}>
                      <Chip
                        icon={<DownloadIcon />}
                        label={plugin.downloads.toLocaleString()}
                        size="small"
                        variant="outlined"
                      />
                    </Grid>
                    <Grid item xs={6}>
                      <Typography variant="caption" color="text.secondary">
                        v{plugin.version}
                      </Typography>
                    </Grid>
                  </Grid>

                  <Typography variant="caption" color="text.secondary" sx={{ mt: 1.5, display: 'block' }}>
                    by {plugin.author.name}
                  </Typography>
                </CardContent>

                <CardActions>
                  <Button size="small" startIcon={<ViewIcon />} onClick={() => handleViewDetails(plugin)}>
                    Details
                  </Button>
                  <Button
                    size="small"
                    variant="contained"
                    startIcon={installing === plugin.name ? <UpdateIcon /> : <DownloadIcon />}
                    onClick={() => handleInstall(plugin.name)}
                    disabled={installing === plugin.name}
                  >
                    {installing === plugin.name ? 'Installing...' : 'Install'}
                  </Button>
                  {plugin.repository && (
                    <IconButton
                      size="small"
                      component="a"
                      href={plugin.repository}
                      target="_blank"
                      rel="noopener noreferrer"
                    >
                      <GitHubIcon />
                    </IconButton>
                  )}
                </CardActions>

                {installing === plugin.name && (
                  <LinearProgress variant="determinate" value={installProgress} />
                )}
              </Card>
            </Grid>
          );
        })}
      </Grid>

      {/* Plugin Details Dialog */}
      <Dialog open={detailsOpen} onClose={() => setDetailsOpen(false)} maxWidth="lg" fullWidth>
        {selectedPlugin && (
          <>
            <DialogTitle>
              <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'start' }}>
                <Box>
                  <Typography variant="h5">{selectedPlugin.name}</Typography>
                  <Typography variant="body2" color="text.secondary">
                    by {selectedPlugin.author.name} • v{selectedPlugin.version}
                  </Typography>
                </Box>
                <Stack direction="row" spacing={1}>
                  <Chip label={selectedPlugin.category} color="primary" />
                  <Chip label={selectedPlugin.language} icon={<CodeIcon />} />
                  <Chip
                    label={`Security: ${selectedPlugin.securityScore}`}
                    icon={<SecurityIcon />}
                    color={selectedPlugin.securityScore >= 70 ? 'success' : 'warning'}
                  />
                </Stack>
              </Box>
            </DialogTitle>

            <DialogContent>
              <Tabs value={activeTab} onChange={(_, v) => setActiveTab(v)} sx={{ mb: 3 }}>
                <Tab label="Overview" />
                <Tab label="Reviews" icon={<Badge badgeContent={reviews.length} color="primary" />} />
                <Tab label="Security" />
                <Tab label="Versions" />
              </Tabs>

              {activeTab === 0 && (
                <Box>
                  <Typography variant="body1" paragraph>
                    {selectedPlugin.description}
                  </Typography>

                  <Box sx={{ display: 'flex', gap: 1, mb: 3 }}>
                    {selectedPlugin.tags.map((tag) => (
                      <Chip key={tag} label={tag} size="small" />
                    ))}
                  </Box>

                  <Grid container spacing={2} sx={{ mb: 3 }}>
                    <Grid item xs={3}>
                      <Paper variant="outlined" sx={{ p: 2, textAlign: 'center' }}>
                        <Typography variant="caption" color="text.secondary">
                          Downloads
                        </Typography>
                        <Typography variant="h6">
                          {selectedPlugin.downloads.toLocaleString()}
                        </Typography>
                      </Paper>
                    </Grid>
                    <Grid item xs={3}>
                      <Paper variant="outlined" sx={{ p: 2, textAlign: 'center' }}>
                        <Typography variant="caption" color="text.secondary">
                          Rating
                        </Typography>
                        <Box sx={{ mt: 0.5 }}>
                          <Rating value={selectedPlugin.rating} readOnly size="small" precision={0.5} />
                        </Box>
                      </Paper>
                    </Grid>
                    <Grid item xs={3}>
                      <Paper variant="outlined" sx={{ p: 2, textAlign: 'center' }}>
                        <Typography variant="caption" color="text.secondary">
                          Reviews
                        </Typography>
                        <Typography variant="h6">{selectedPlugin.reviewsCount}</Typography>
                      </Paper>
                    </Grid>
                    <Grid item xs={3}>
                      <Paper variant="outlined" sx={{ p: 2, textAlign: 'center' }}>
                        <Typography variant="caption" color="text.secondary">
                          License
                        </Typography>
                        <Typography variant="h6">{selectedPlugin.license}</Typography>
                      </Paper>
                    </Grid>
                  </Grid>

                  {selectedPlugin.repository && (
                    <Typography variant="body2" sx={{ mb: 1 }}>
                      <strong>Repository:</strong>{' '}
                      <a href={selectedPlugin.repository} target="_blank" rel="noopener noreferrer">
                        {selectedPlugin.repository}
                      </a>
                    </Typography>
                  )}
                  {selectedPlugin.homepage && (
                    <Typography variant="body2">
                      <strong>Homepage:</strong>{' '}
                      <a href={selectedPlugin.homepage} target="_blank" rel="noopener noreferrer">
                        {selectedPlugin.homepage}
                      </a>
                    </Typography>
                  )}
                </Box>
              )}

              {activeTab === 1 && (
                <Box>
                  {reviews.length === 0 ? (
                    <Alert severity="info">No reviews yet. Be the first to review!</Alert>
                  ) : (
                    <List>
                      {reviews.map((review) => (
                        <React.Fragment key={review.id}>
                          <ListItem alignItems="flex-start">
                            <Box sx={{ width: '100%' }}>
                              <Box sx={{ display: 'flex', justifyContent: 'space-between', mb: 1 }}>
                                <Box sx={{ display: 'flex', gap: 1, alignItems: 'center' }}>
                                  {review.userAvatar && <Avatar src={review.userAvatar} sx={{ width: 32, height: 32 }} />}
                                  <Box>
                                    <Typography variant="subtitle2">{review.userName}</Typography>
                                    {review.verified && (
                                      <Chip label="Verified" size="small" color="success" sx={{ height: 16 }} />
                                    )}
                                  </Box>
                                </Box>
                                <Typography variant="caption" color="text.secondary">
                                  {new Date(review.createdAt).toLocaleDateString()}
                                </Typography>
                              </Box>
                              <Box sx={{ mb: 1 }}>
                                <Rating value={review.rating} readOnly size="small" />
                                {review.title && (
                                  <Typography variant="subtitle2" sx={{ mt: 0.5 }}>
                                    {review.title}
                                  </Typography>
                                )}
                              </Box>
                              <Typography variant="body2" sx={{ mb: 1 }}>
                                {review.comment}
                              </Typography>
                              <Box sx={{ display: 'flex', gap: 2 }}>
                                <Button
                                  size="small"
                                  startIcon={<ThumbUpIcon />}
                                  onClick={() => handleVoteReview(review.id, true)}
                                >
                                  Helpful ({review.helpfulCount})
                                </Button>
                                <Button
                                  size="small"
                                  startIcon={<ThumbDownIcon />}
                                  onClick={() => handleVoteReview(review.id, false)}
                                >
                                  Not helpful ({review.unhelpfulCount})
                                </Button>
                              </Box>
                              {review.authorResponse && (
                                <Box sx={{ mt: 2, ml: 4, p: 2, bgcolor: 'action.hover', borderRadius: 1 }}>
                                  <Typography variant="caption" color="primary" fontWeight="bold">
                                    Author Response
                                  </Typography>
                                  <Typography variant="body2">{review.authorResponse.text}</Typography>
                                </Box>
                              )}
                            </Box>
                          </ListItem>
                          <Divider component="li" />
                        </React.Fragment>
                      ))}
                    </List>
                  )}
                  <Box sx={{ mt: 2 }}>
                    <Button variant="outlined" onClick={() => setReviewDialogOpen(true)}>
                      Write a Review
                    </Button>
                  </Box>
                </Box>
              )}

              {activeTab === 2 && (
                <Box>
                  {securityScan ? (
                    <>
                      <Box sx={{ mb: 3 }}>
                        <Typography variant="h6" gutterBottom>
                          Security Score: {securityScan.score}/100
                        </Typography>
                        <LinearProgress
                          variant="determinate"
                          value={securityScan.score}
                          color={securityScan.status === 'pass' ? 'success' : securityScan.status === 'warning' ? 'warning' : 'error'}
                          sx={{ height: 10, borderRadius: 5 }}
                        />
                      </Box>

                      {securityScan.findings.length > 0 ? (
                        <List>
                          {securityScan.findings.map((finding, index) => (
                            <ListItem key={index}>
                              <ListItemText
                                primary={
                                  <Box sx={{ display: 'flex', gap: 1, alignItems: 'center' }}>
                                    <Chip
                                      label={finding.severity.toUpperCase()}
                                      size="small"
                                      color={
                                        finding.severity === 'critical' || finding.severity === 'high'
                                          ? 'error'
                                          : finding.severity === 'medium'
                                          ? 'warning'
                                          : 'info'
                                      }
                                    />
                                    <Typography variant="subtitle2">{finding.title}</Typography>
                                  </Box>
                                }
                                secondary={finding.description}
                              />
                            </ListItem>
                          ))}
                        </List>
                      ) : (
                        <Alert severity="success">No security issues found</Alert>
                      )}
                    </>
                  ) : (
                    <Alert severity="info">Security scan not available</Alert>
                  )}
                </Box>
              )}

              {activeTab === 3 && (
                <Box>
                  <List>
                    {selectedPlugin.versions.map((version) => (
                      <ListItem
                        key={version.version}
                        secondaryAction={
                          <Button
                            size="small"
                            variant="outlined"
                            onClick={() => handleInstall(selectedPlugin.name, version.version)}
                            disabled={version.yanked}
                          >
                            {version.yanked ? 'Yanked' : 'Install'}
                          </Button>
                        }
                      >
                        <ListItemText
                          primary={
                            <Box sx={{ display: 'flex', gap: 1, alignItems: 'center' }}>
                              <Typography variant="subtitle1">{version.version}</Typography>
                              {version.version === selectedPlugin.version && (
                                <Chip label="Latest" size="small" color="primary" />
                              )}
                              {version.yanked && <Chip label="Yanked" size="small" color="error" />}
                            </Box>
                          }
                          secondary={`Published: ${new Date(version.publishedAt).toLocaleDateString()}`}
                        />
                      </ListItem>
                    ))}
                  </List>
                </Box>
              )}
            </DialogContent>

            <DialogActions>
              <Button onClick={() => setDetailsOpen(false)}>Close</Button>
              <Button
                variant="contained"
                startIcon={<DownloadIcon />}
                onClick={() => {
                  handleInstall(selectedPlugin.name);
                  setDetailsOpen(false);
                }}
              >
                Install Latest
              </Button>
            </DialogActions>
          </>
        )}
      </Dialog>
    </Box>
  );
};

export default PluginRegistryPage;
