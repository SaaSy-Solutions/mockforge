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
  Snackbar,
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
  CheckCircle as CheckCircleIcon,
  Warning as WarningIcon,
  Error as ErrorIcon,
  ThumbUp as ThumbUpIcon,
  ThumbDown as ThumbDownIcon,
} from '@mui/icons-material';
import { useNavigate } from 'react-router-dom';
import { authenticatedFetch } from '../utils/apiClient';
import { useAuthStore } from '../stores/useAuthStore';
import { PublishPluginModal } from '../components/marketplace/PublishPluginModal';
import { MarketplaceTabs } from '../components/marketplace/MarketplaceTabs';

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

interface ReviewStats {
  averageRating: number;
  totalReviews: number;
  ratingDistribution: Record<string, number>;
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
  size?: number;
  minMockforgeVersion?: string | null;
  dependencies?: Record<string, string>;
  downloads?: number;
}

const formatFileSize = (bytes: number): string => {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
};

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
  } | null;
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
  const navigate = useNavigate();
  const currentUser = useAuthStore((s) => s.user);
  const isAdmin = currentUser?.role === 'admin';

  const [plugins, setPlugins] = useState<Plugin[]>([]);
  const [filteredPlugins, setFilteredPlugins] = useState<Plugin[]>([]);
  const [selectedPlugin, setSelectedPlugin] = useState<Plugin | null>(null);
  const [reviews, setReviews] = useState<Review[]>([]);
  const [reviewStats, setReviewStats] = useState<ReviewStats | null>(null);
  const [securityScan, setSecurityScan] = useState<SecurityScanResult | null>(null);
  const [detailsOpen, setDetailsOpen] = useState(false);
  const [reviewDialogOpen, setReviewDialogOpen] = useState(false);
  const [activeTab, setActiveTab] = useState(0);
  const [pluginBadges, setPluginBadges] = useState<Record<string, string[]>>({});
  const [reviewForm, setReviewForm] = useState<{ rating: number; title: string; comment: string }>(
    { rating: 5, title: '', comment: '' }
  );
  const [reviewSubmitting, setReviewSubmitting] = useState(false);
  const [reviewError, setReviewError] = useState<string | null>(null);
  const [copyFeedback, setCopyFeedback] = useState<string | null>(null);
  const [pageLoading, setPageLoading] = useState(false);
  const [page, setPage] = useState(0);
  const [totalPlugins, setTotalPlugins] = useState(0);
  const [publishOpen, setPublishOpen] = useState(false);
  const [yankingVersion, setYankingVersion] = useState<string | null>(null);
  const [verifyBusy, setVerifyBusy] = useState(false);
  const [takedownBusy, setTakedownBusy] = useState(false);
  const [takedownDialogOpen, setTakedownDialogOpen] = useState(false);
  const [takedownReason, setTakedownReason] = useState('');
  // Track the name of the last plugin we took down so the snackbar can
  // offer an Undo action — useful since taken-down plugins disappear from
  // search and there's no other UI surface to find them again.
  const [recentlyTakenDown, setRecentlyTakenDown] = useState<string | null>(null);
  const [respondingToReview, setRespondingToReview] = useState<Review | null>(null);
  const [responseDraft, setResponseDraft] = useState('');
  const [responseSubmitting, setResponseSubmitting] = useState(false);
  const [responseError, setResponseError] = useState<string | null>(null);

  const PAGE_SIZE = 24;

  // Filters
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedCategory, setSelectedCategory] = useState('all');
  const [selectedLanguage, setSelectedLanguage] = useState('all');
  const [sortBy, setSortBy] = useState('popular');
  const [minRating, setMinRating] = useState(0);
  const [minSecurityScore, setMinSecurityScore] = useState(0);


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
    // Reset paging whenever a server-side filter changes. Search query is
    // debounced separately below so each keystroke doesn't fire a request.
    setPage(0);
    loadPlugins(0, false);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sortBy, selectedLanguage, selectedCategory]);

  useEffect(() => {
    const handle = setTimeout(() => {
      setPage(0);
      loadPlugins(0, false);
    }, 300);
    return () => clearTimeout(handle);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [searchQuery]);

  useEffect(() => {
    filterPlugins();
  }, [plugins, minRating, minSecurityScore]);

  const loadPlugins = async (nextPage: number, append: boolean) => {
    setPageLoading(true);
    try {
      const trimmedQuery = searchQuery.trim();
      const response = await authenticatedFetch('/api/v1/plugins/search', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          query: trimmedQuery.length > 0 ? trimmedQuery : null,
          category: selectedCategory === 'all' ? null : selectedCategory,
          language: selectedLanguage === 'all' ? null : selectedLanguage,
          tags: [],
          sort: sortBy,
          page: nextPage,
          perPage: PAGE_SIZE,
        }),
      });

      if (response.ok) {
        const data = await response.json();
        const loaded: Plugin[] = data.plugins || [];
        setTotalPlugins(typeof data.total === 'number' ? data.total : loaded.length);
        setPlugins((prev) => (append ? [...prev, ...loaded] : loaded));
        loadBadges(loaded);
      }
    } catch (error) {
      console.error('Failed to load plugins:', error);
    } finally {
      setPageLoading(false);
    }
  };

  const handleLoadMore = () => {
    const next = page + 1;
    setPage(next);
    loadPlugins(next, true);
  };

  const loadBadges = async (list: Plugin[]) => {
    // Fetch badges per plugin in parallel; failures are non-fatal.
    const entries = await Promise.all(
      list.map(async (p) => {
        try {
          const resp = await authenticatedFetch(`/api/v1/plugins/${p.name}/badges`);
          if (!resp.ok) return [p.name, []] as const;
          const data = await resp.json();
          return [p.name, (data.badges as string[]) || []] as const;
        } catch {
          return [p.name, []] as const;
        }
      })
    );
    setPluginBadges(Object.fromEntries(entries));
  };

  const filterPlugins = () => {
    // Search query, category, language, and sort are applied server-side via
    // /api/v1/plugins/search so they reflect the entire catalog rather than
    // just the currently-loaded page. Min-rating and min-security are applied
    // here because the backend search API doesn't accept those filters yet.
    let filtered = [...plugins];

    if (minRating > 0) {
      filtered = filtered.filter((p) => p.rating >= minRating);
    }

    if (minSecurityScore > 0) {
      filtered = filtered.filter((p) => p.securityScore >= minSecurityScore);
    }

    setFilteredPlugins(filtered);
  };

  const handleViewDetails = async (plugin: Plugin) => {
    setSelectedPlugin(plugin);
    setDetailsOpen(true);
    setActiveTab(0);

    // Load reviews
    try {
      const response = await authenticatedFetch(`/api/v1/plugins/${plugin.name}/reviews`);
      if (response.ok) {
        const data = await response.json();
        setReviews(data.reviews || []);
        setReviewStats(data.stats || null);
      }
    } catch (error) {
      console.error('Failed to load reviews:', error);
    }

    // Load security scan
    try {
      const response = await authenticatedFetch(`/api/v1/plugins/${plugin.name}/security`);
      if (response.ok) {
        const data = await response.json();
        setSecurityScan(data);
      }
    } catch (error) {
      console.error('Failed to load security scan:', error);
    }
  };

  const handleInstall = async (plugin: Plugin, version?: VersionInfo) => {
    // The cloud registry UI cannot reach a local MockForge server, so the
    // install action copies a CLI command that the user runs in their own
    // environment. Local admin UIs use a separate /api/plugins/install path.
    const selectedVersion = version || plugin.versions[0];
    const versionStr = selectedVersion?.version || plugin.version;
    const command = `mockforge plugin install ${plugin.name}@${versionStr}`;
    try {
      await navigator.clipboard.writeText(command);
      setCopyFeedback(`Copied: ${command}`);
    } catch (error) {
      console.error('Failed to copy install command:', error);
      setCopyFeedback(`Run: ${command}`);
    }
    setTimeout(() => setCopyFeedback(null), 4000);
  };

  const handleVoteReview = async (reviewId: string, helpful: boolean) => {
    try {
      if (!selectedPlugin) {
        return;
      }
      await authenticatedFetch(`/api/v1/plugins/${selectedPlugin.name}/reviews/${reviewId}/vote`, {
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

  const openReviewDialog = () => {
    setReviewForm({ rating: 5, title: '', comment: '' });
    setReviewError(null);
    setReviewDialogOpen(true);
  };

  const handleSubmitReview = async () => {
    if (!selectedPlugin) return;
    setReviewError(null);
    if (reviewForm.comment.trim().length < 10) {
      setReviewError('Comment must be at least 10 characters.');
      return;
    }
    if (reviewForm.rating < 1 || reviewForm.rating > 5) {
      setReviewError('Please choose a rating between 1 and 5 stars.');
      return;
    }
    setReviewSubmitting(true);
    try {
      const response = await authenticatedFetch(
        `/api/v1/plugins/${selectedPlugin.name}/reviews`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            version: selectedPlugin.version,
            rating: reviewForm.rating,
            title: reviewForm.title.trim() || null,
            comment: reviewForm.comment.trim(),
          }),
        }
      );
      const data = await response.json().catch(() => ({}));
      if (!response.ok || data?.success === false) {
        throw new Error(data?.error || data?.message || 'Failed to submit review');
      }
      setReviewDialogOpen(false);
      handleViewDetails(selectedPlugin);
    } catch (error) {
      setReviewError(error instanceof Error ? error.message : 'Failed to submit review');
    } finally {
      setReviewSubmitting(false);
    }
  };

  const refreshBadgesFor = async (name: string) => {
    try {
      const resp = await authenticatedFetch(`/api/v1/plugins/${encodeURIComponent(name)}/badges`);
      if (!resp.ok) return;
      const data = await resp.json();
      setPluginBadges((prev) => ({ ...prev, [name]: (data.badges as string[]) || [] }));
    } catch {
      /* non-fatal */
    }
  };

  const handleYank = async (plugin: Plugin, version: string) => {
    if (!window.confirm(`Yank ${plugin.name}@${version}? Installed users keep the file, but new installs will fail.`)) {
      return;
    }
    setYankingVersion(version);
    try {
      const resp = await authenticatedFetch(
        `/api/v1/plugins/${encodeURIComponent(plugin.name)}/versions/${encodeURIComponent(version)}/yank`,
        { method: 'DELETE' }
      );
      if (!resp.ok) {
        const err = await resp.json().catch(() => ({}));
        throw new Error(err?.error || err?.message || `HTTP ${resp.status}`);
      }
      setCopyFeedback(`Yanked ${plugin.name}@${version}`);
      // Refresh detail view so the Yanked chip appears
      await handleViewDetails(plugin);
    } catch (error) {
      const msg = error instanceof Error ? error.message : 'Yank failed';
      setCopyFeedback(`Yank failed: ${msg}`);
    } finally {
      setYankingVersion(null);
      setTimeout(() => setCopyFeedback(null), 4000);
    }
  };

  const handleToggleVerify = async (plugin: Plugin) => {
    const currentlyVerified = (pluginBadges[plugin.name] || []).includes('verified');
    setVerifyBusy(true);
    try {
      const resp = await authenticatedFetch(
        `/api/v1/admin/plugins/${encodeURIComponent(plugin.name)}/verify`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ verified: !currentlyVerified }),
        }
      );
      if (!resp.ok) {
        const err = await resp.json().catch(() => ({}));
        throw new Error(err?.error || err?.message || `HTTP ${resp.status}`);
      }
      setCopyFeedback(
        currentlyVerified ? `Unverified ${plugin.name}` : `Verified ${plugin.name}`
      );
      await refreshBadgesFor(plugin.name);
    } catch (error) {
      const msg = error instanceof Error ? error.message : 'Verify action failed';
      setCopyFeedback(msg);
    } finally {
      setVerifyBusy(false);
      setTimeout(() => setCopyFeedback(null), 4000);
    }
  };

  const handleTakedown = async () => {
    if (!selectedPlugin) return;
    setTakedownBusy(true);
    try {
      const resp = await authenticatedFetch(
        `/api/v1/admin/plugins/${encodeURIComponent(selectedPlugin.name)}/takedown`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ reason: takedownReason.trim() || null }),
        }
      );
      if (!resp.ok) {
        const err = await resp.json().catch(() => ({}));
        throw new Error(err?.error || err?.message || `HTTP ${resp.status}`);
      }
      setRecentlyTakenDown(selectedPlugin.name);
      setCopyFeedback(`Took down ${selectedPlugin.name}`);
      setTakedownDialogOpen(false);
      setTakedownReason('');
      // Plugin is now hidden from search; close the dialog and drop it
      // from the local list so the admin sees the moderation took effect.
      setDetailsOpen(false);
      setPlugins((prev) => prev.filter((p) => p.name !== selectedPlugin.name));
    } catch (error) {
      const msg = error instanceof Error ? error.message : 'Takedown failed';
      setCopyFeedback(msg);
    } finally {
      setTakedownBusy(false);
      setTimeout(() => setCopyFeedback(null), 4000);
    }
  };

  const handleRestore = async (pluginName: string) => {
    setTakedownBusy(true);
    try {
      const resp = await authenticatedFetch(
        `/api/v1/admin/plugins/${encodeURIComponent(pluginName)}/restore`,
        { method: 'POST' }
      );
      if (!resp.ok) {
        const err = await resp.json().catch(() => ({}));
        throw new Error(err?.error || err?.message || `HTTP ${resp.status}`);
      }
      setCopyFeedback(`Restored ${pluginName}`);
      setRecentlyTakenDown(null);
      loadPlugins(0, false);
      setPage(0);
    } catch (error) {
      const msg = error instanceof Error ? error.message : 'Restore failed';
      setCopyFeedback(msg);
    } finally {
      setTakedownBusy(false);
      setTimeout(() => setCopyFeedback(null), 4000);
    }
  };

  const openRespondDialog = (review: Review) => {
    setRespondingToReview(review);
    setResponseDraft(review.authorResponse?.text || '');
    setResponseError(null);
  };

  const closeRespondDialog = () => {
    if (responseSubmitting) return;
    setRespondingToReview(null);
    setResponseDraft('');
    setResponseError(null);
  };

  const handleSubmitAuthorResponse = async () => {
    if (!selectedPlugin || !respondingToReview) return;
    const trimmed = responseDraft.trim();
    if (trimmed.length === 0) {
      setResponseError('Response cannot be empty.');
      return;
    }
    if (trimmed.length > 5000) {
      setResponseError('Response must be 5000 characters or fewer.');
      return;
    }
    setResponseSubmitting(true);
    setResponseError(null);
    try {
      const resp = await authenticatedFetch(
        `/api/v1/plugins/${encodeURIComponent(selectedPlugin.name)}/reviews/${encodeURIComponent(respondingToReview.id)}/respond`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ text: trimmed }),
        }
      );
      const data = await resp.json().catch(() => ({}));
      if (!resp.ok || data?.success === false) {
        throw new Error(data?.error || data?.message || `HTTP ${resp.status}`);
      }
      setRespondingToReview(null);
      setResponseDraft('');
      handleViewDetails(selectedPlugin);
    } catch (error) {
      setResponseError(error instanceof Error ? error.message : 'Failed to post response');
    } finally {
      setResponseSubmitting(false);
    }
  };

  const handleClearAuthorResponse = async (review: Review) => {
    if (!selectedPlugin) return;
    if (!window.confirm('Remove your response to this review?')) return;
    try {
      const resp = await authenticatedFetch(
        `/api/v1/plugins/${encodeURIComponent(selectedPlugin.name)}/reviews/${encodeURIComponent(review.id)}/respond`,
        { method: 'DELETE' }
      );
      if (!resp.ok) {
        const err = await resp.json().catch(() => ({}));
        throw new Error(err?.error || err?.message || `HTTP ${resp.status}`);
      }
      handleViewDetails(selectedPlugin);
    } catch (error) {
      const msg = error instanceof Error ? error.message : 'Failed to remove response';
      setCopyFeedback(msg);
      setTimeout(() => setCopyFeedback(null), 4000);
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
      <MarketplaceTabs />

      {/* Header */}
      <Box
        sx={{
          mb: 4,
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'start',
          gap: 2,
          flexWrap: 'wrap',
        }}
      >
        <Box>
          <Typography variant="h4" gutterBottom>
            Plugin Registry
          </Typography>
          <Typography variant="body1" color="text.secondary">
            Discover and install plugins from the MockForge ecosystem
          </Typography>
        </Box>
        <Stack direction="row" spacing={1} alignItems="center">
          {isAdmin && (
            <Button
              variant="outlined"
              size="small"
              onClick={() => navigate('/plugin-registry/moderation')}
            >
              Moderation
            </Button>
          )}
          {currentUser && (
            <Button variant="contained" onClick={() => setPublishOpen(true)}>
              Publish Plugin
            </Button>
          )}
        </Stack>
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
      <Box sx={{ mb: 2, display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <Typography variant="body2" color="text.secondary">
          {filteredPlugins.length} of {totalPlugins || plugins.length} plugin
          {totalPlugins === 1 ? '' : 's'} {filteredPlugins.length !== plugins.length && '(filtered) '}
          loaded
        </Typography>
        {pageLoading && (
          <Typography variant="caption" color="text.secondary">
            Loading…
          </Typography>
        )}
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

                  {(pluginBadges[plugin.name] || []).length > 0 && (
                    <Box sx={{ mb: 2, display: 'flex', flexWrap: 'wrap', gap: 0.5 }}>
                      {(pluginBadges[plugin.name] || []).map((badge) => (
                        <Chip
                          key={badge}
                          label={badge.replace(/-/g, ' ')}
                          size="small"
                          color={
                            badge === 'official' || badge === 'verified' || badge === 'signed'
                              ? 'success'
                              : badge === 'popular' || badge === 'trending'
                              ? 'info'
                              : badge === 'highly-rated'
                              ? 'warning'
                              : 'default'
                          }
                          sx={{ textTransform: 'capitalize' }}
                        />
                      ))}
                    </Box>
                  )}

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
                  <Tooltip title="Copy CLI install command to clipboard">
                    <Button
                      size="small"
                      variant="contained"
                      startIcon={<DownloadIcon />}
                      onClick={() => handleInstall(plugin)}
                    >
                      Install
                    </Button>
                  </Tooltip>
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
              </Card>
            </Grid>
          );
        })}
      </Grid>

      {plugins.length < totalPlugins && (
        <Box sx={{ mt: 3, display: 'flex', justifyContent: 'center' }}>
          <Button
            variant="outlined"
            onClick={handleLoadMore}
            disabled={pageLoading}
          >
            {pageLoading ? 'Loading…' : `Load more (${totalPlugins - plugins.length} remaining)`}
          </Button>
        </Box>
      )}

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
                <Stack direction="row" spacing={1} alignItems="center">
                  <Chip label={selectedPlugin.category} color="primary" />
                  <Chip label={selectedPlugin.language} icon={<CodeIcon />} />
                  <Chip
                    label={`Security: ${selectedPlugin.securityScore}`}
                    icon={<SecurityIcon />}
                    color={selectedPlugin.securityScore >= 70 ? 'success' : 'warning'}
                  />
                  {isAdmin && (
                    <>
                      <Button
                        size="small"
                        variant="outlined"
                        color={(pluginBadges[selectedPlugin.name] || []).includes('verified') ? 'warning' : 'success'}
                        disabled={verifyBusy}
                        onClick={() => handleToggleVerify(selectedPlugin)}
                      >
                        {(pluginBadges[selectedPlugin.name] || []).includes('verified')
                          ? 'Unverify'
                          : 'Verify'}
                      </Button>
                      <Button
                        size="small"
                        variant="outlined"
                        color="error"
                        disabled={takedownBusy}
                        onClick={() => setTakedownDialogOpen(true)}
                      >
                        Take down
                      </Button>
                    </>
                  )}
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
                  {reviewStats && reviewStats.totalReviews > 0 && (
                    <Paper variant="outlined" sx={{ p: 2, mb: 2 }}>
                      <Box sx={{ display: 'flex', alignItems: 'center', gap: 3, mb: 2 }}>
                        <Box>
                          <Typography variant="h3" component="div">
                            {reviewStats.averageRating.toFixed(1)}
                          </Typography>
                          <Rating
                            value={reviewStats.averageRating}
                            readOnly
                            precision={0.1}
                            size="small"
                          />
                          <Typography variant="caption" color="text.secondary" display="block">
                            {reviewStats.totalReviews} review
                            {reviewStats.totalReviews !== 1 ? 's' : ''}
                          </Typography>
                        </Box>
                        <Box sx={{ flexGrow: 1 }}>
                          {[5, 4, 3, 2, 1].map((stars) => {
                            const count = reviewStats.ratingDistribution?.[String(stars)] || 0;
                            const pct = reviewStats.totalReviews
                              ? (count / reviewStats.totalReviews) * 100
                              : 0;
                            return (
                              <Box
                                key={stars}
                                sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 0.5 }}
                              >
                                <Typography variant="caption" sx={{ width: 16 }}>
                                  {stars}
                                </Typography>
                                <StarIcon fontSize="small" color="warning" />
                                <LinearProgress
                                  variant="determinate"
                                  value={pct}
                                  sx={{ flexGrow: 1, height: 8, borderRadius: 4 }}
                                />
                                <Typography
                                  variant="caption"
                                  color="text.secondary"
                                  sx={{ width: 28, textAlign: 'right' }}
                                >
                                  {count}
                                </Typography>
                              </Box>
                            );
                          })}
                        </Box>
                      </Box>
                    </Paper>
                  )}
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
                              <Box sx={{ display: 'flex', gap: 2, flexWrap: 'wrap' }}>
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
                                {selectedPlugin &&
                                  currentUser?.username === selectedPlugin.author.name && (
                                    <Button
                                      size="small"
                                      onClick={() => openRespondDialog(review)}
                                    >
                                      {review.authorResponse ? 'Edit response' : 'Respond as author'}
                                    </Button>
                                  )}
                                {review.authorResponse &&
                                  selectedPlugin &&
                                  currentUser?.username === selectedPlugin.author.name && (
                                    <Button
                                      size="small"
                                      color="error"
                                      onClick={() => handleClearAuthorResponse(review)}
                                    >
                                      Remove response
                                    </Button>
                                  )}
                              </Box>
                              {review.authorResponse && (
                                <Box
                                  sx={{
                                    mt: 2,
                                    ml: 4,
                                    p: 2,
                                    bgcolor: 'action.hover',
                                    borderRadius: 1,
                                  }}
                                >
                                  <Typography variant="caption" color="primary" fontWeight="bold">
                                    Author response ·{' '}
                                    {new Date(review.authorResponse.createdAt).toLocaleDateString()}
                                  </Typography>
                                  <Typography variant="body2">
                                    {review.authorResponse.text}
                                  </Typography>
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
                    <Button variant="outlined" onClick={openReviewDialog}>
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
                    {selectedPlugin.versions.map((version) => {
                      const isAuthor =
                        !!currentUser?.username &&
                        currentUser.username === selectedPlugin.author.name;
                      return (
                        <ListItem
                          key={version.version}
                          secondaryAction={
                            <Stack direction="row" spacing={1}>
                              {isAuthor && !version.yanked && (
                                <Button
                                  size="small"
                                  color="error"
                                  variant="outlined"
                                  disabled={yankingVersion === version.version}
                                  onClick={() => handleYank(selectedPlugin, version.version)}
                                >
                                  {yankingVersion === version.version ? 'Yanking…' : 'Yank'}
                                </Button>
                              )}
                              <Button
                                size="small"
                                variant="outlined"
                                onClick={() => handleInstall(selectedPlugin, version)}
                                disabled={version.yanked}
                              >
                                {version.yanked ? 'Yanked' : 'Install'}
                              </Button>
                            </Stack>
                          }
                        >
                          <ListItemText
                            primary={
                              <Box sx={{ display: 'flex', gap: 1, alignItems: 'center', flexWrap: 'wrap' }}>
                                <Typography variant="subtitle1">{version.version}</Typography>
                                {version.version === selectedPlugin.version && (
                                  <Chip label="Latest" size="small" color="primary" />
                                )}
                                {version.yanked && <Chip label="Yanked" size="small" color="error" />}
                                {version.minMockforgeVersion && (
                                  <Tooltip title="Minimum MockForge version required to run this plugin">
                                    <Chip
                                      label={`MockForge ≥ ${version.minMockforgeVersion}`}
                                      size="small"
                                      variant="outlined"
                                    />
                                  </Tooltip>
                                )}
                                {typeof version.size === 'number' && version.size > 0 && (
                                  <Chip label={formatFileSize(version.size)} size="small" variant="outlined" />
                                )}
                                {typeof version.downloads === 'number' && version.downloads > 0 && (
                                  <Chip
                                    icon={<DownloadIcon />}
                                    label={version.downloads.toLocaleString()}
                                    size="small"
                                    variant="outlined"
                                  />
                                )}
                              </Box>
                            }
                            secondary={
                              <Box component="span" sx={{ display: 'block' }}>
                                <Typography variant="caption" color="text.secondary" component="span">
                                  Published {new Date(version.publishedAt).toLocaleDateString()}
                                </Typography>
                                {version.checksum && (
                                  <Typography
                                    variant="caption"
                                    color="text.secondary"
                                    component="span"
                                    sx={{ display: 'block', fontFamily: 'monospace', mt: 0.25, wordBreak: 'break-all' }}
                                  >
                                    sha256: {version.checksum}
                                  </Typography>
                                )}
                                {version.dependencies && Object.keys(version.dependencies).length > 0 && (
                                  <Box component="span" sx={{ display: 'block', mt: 0.5 }}>
                                    <Typography variant="caption" color="text.secondary" component="span">
                                      Depends on:{' '}
                                    </Typography>
                                    {Object.entries(version.dependencies).map(([dep, range]) => (
                                      <Chip
                                        key={dep}
                                        label={`${dep} ${range}`}
                                        size="small"
                                        variant="outlined"
                                        sx={{ mr: 0.5, mt: 0.25 }}
                                      />
                                    ))}
                                  </Box>
                                )}
                              </Box>
                            }
                          />
                        </ListItem>
                      );
                    })}
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
                  handleInstall(selectedPlugin);
                  setDetailsOpen(false);
                }}
              >
                Install Latest
              </Button>
            </DialogActions>
          </>
        )}
      </Dialog>

      {/* Submit Review Dialog */}
      <Dialog
        open={reviewDialogOpen}
        onClose={() => (!reviewSubmitting ? setReviewDialogOpen(false) : undefined)}
        maxWidth="sm"
        fullWidth
      >
        <DialogTitle>
          Write a review{selectedPlugin ? ` for ${selectedPlugin.name}` : ''}
        </DialogTitle>
        <DialogContent>
          <Stack spacing={2} sx={{ mt: 1 }}>
            <Box>
              <Typography variant="caption" color="text.secondary" display="block" gutterBottom>
                Rating
              </Typography>
              <Rating
                value={reviewForm.rating}
                onChange={(_, value) =>
                  setReviewForm((prev) => ({ ...prev, rating: value || 0 }))
                }
              />
            </Box>
            <TextField
              label="Title (optional)"
              value={reviewForm.title}
              onChange={(e) =>
                setReviewForm((prev) => ({ ...prev, title: e.target.value.slice(0, 100) }))
              }
              fullWidth
              inputProps={{ maxLength: 100 }}
            />
            <TextField
              label="Comment"
              value={reviewForm.comment}
              onChange={(e) =>
                setReviewForm((prev) => ({ ...prev, comment: e.target.value.slice(0, 5000) }))
              }
              multiline
              minRows={4}
              fullWidth
              required
              helperText={`${reviewForm.comment.length}/5000 · min 10 characters`}
            />
            {reviewError && <Alert severity="error">{reviewError}</Alert>}
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button
            onClick={() => setReviewDialogOpen(false)}
            disabled={reviewSubmitting}
          >
            Cancel
          </Button>
          <Button
            variant="contained"
            onClick={handleSubmitReview}
            disabled={reviewSubmitting || reviewForm.comment.trim().length < 10}
          >
            {reviewSubmitting ? 'Submitting…' : 'Submit Review'}
          </Button>
        </DialogActions>
      </Dialog>

      <Dialog
        open={takedownDialogOpen}
        onClose={() => (!takedownBusy ? setTakedownDialogOpen(false) : undefined)}
        maxWidth="sm"
        fullWidth
      >
        <DialogTitle>
          Take down {selectedPlugin ? selectedPlugin.name : ''}
        </DialogTitle>
        <DialogContent>
          <Stack spacing={2} sx={{ mt: 1 }}>
            <Alert severity="warning">
              The plugin will be hidden from search and detail views. Existing
              installs keep working — restoring it later from the audit log
              undoes the takedown.
            </Alert>
            <TextField
              label="Reason (optional)"
              value={takedownReason}
              onChange={(e) => setTakedownReason(e.target.value.slice(0, 1000))}
              multiline
              minRows={3}
              fullWidth
              helperText="Stored on the plugin row for future moderation review."
            />
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setTakedownDialogOpen(false)} disabled={takedownBusy}>
            Cancel
          </Button>
          <Button
            variant="contained"
            color="error"
            onClick={handleTakedown}
            disabled={takedownBusy}
          >
            {takedownBusy ? 'Taking down…' : 'Take down'}
          </Button>
        </DialogActions>
      </Dialog>

      <Dialog
        open={Boolean(respondingToReview)}
        onClose={closeRespondDialog}
        maxWidth="sm"
        fullWidth
      >
        <DialogTitle>
          {respondingToReview?.authorResponse ? 'Edit your response' : 'Respond to review'}
        </DialogTitle>
        <DialogContent>
          <Stack spacing={2} sx={{ mt: 1 }}>
            {respondingToReview && (
              <Paper variant="outlined" sx={{ p: 2 }}>
                <Typography variant="caption" color="text.secondary">
                  {respondingToReview.userName} · {respondingToReview.rating}★
                </Typography>
                {respondingToReview.title && (
                  <Typography variant="subtitle2" sx={{ mt: 0.5 }}>
                    {respondingToReview.title}
                  </Typography>
                )}
                <Typography variant="body2" sx={{ mt: 0.5 }}>
                  {respondingToReview.comment}
                </Typography>
              </Paper>
            )}
            <TextField
              label="Your response"
              value={responseDraft}
              onChange={(e) => setResponseDraft(e.target.value.slice(0, 5000))}
              multiline
              minRows={4}
              fullWidth
              required
              helperText={`${responseDraft.length}/5000`}
            />
            {responseError && <Alert severity="error">{responseError}</Alert>}
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={closeRespondDialog} disabled={responseSubmitting}>
            Cancel
          </Button>
          <Button
            variant="contained"
            onClick={handleSubmitAuthorResponse}
            disabled={responseSubmitting || responseDraft.trim().length === 0}
          >
            {responseSubmitting
              ? 'Posting…'
              : respondingToReview?.authorResponse
              ? 'Update response'
              : 'Post response'}
          </Button>
        </DialogActions>
      </Dialog>

      <PublishPluginModal
        open={publishOpen}
        onClose={() => setPublishOpen(false)}
        onSuccess={() => {
          setPublishOpen(false);
          setCopyFeedback('Plugin published — refreshing catalog…');
          loadPlugins(0, false);
          setPage(0);
          setTimeout(() => setCopyFeedback(null), 4000);
        }}
      />

      <Snackbar
        open={Boolean(copyFeedback)}
        autoHideDuration={recentlyTakenDown ? 10000 : 4000}
        onClose={() => {
          setCopyFeedback(null);
          setRecentlyTakenDown(null);
        }}
        anchorOrigin={{ vertical: 'bottom', horizontal: 'center' }}
        message={copyFeedback || ''}
        action={
          recentlyTakenDown ? (
            <Button
              size="small"
              color="inherit"
              disabled={takedownBusy}
              onClick={() => handleRestore(recentlyTakenDown)}
            >
              Undo
            </Button>
          ) : undefined
        }
      />
    </Box>
  );
};

export default PluginRegistryPage;
