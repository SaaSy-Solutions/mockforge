/**
 * Scenario Marketplace Page
 *
 * Browse, search, and install mock scenarios from the marketplace.
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
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  FormControl,
  InputLabel,
  Select,
  MenuItem,
  List,
  ListItem,
  ListItemText,
  Divider,
  Alert,
  Snackbar,
  Link,
} from '@mui/material';
import {
  Search as SearchIcon,
  Star as StarIcon,
  StarBorder as StarBorderIcon,
  ThumbUpAltOutlined as ThumbUpOutlinedIcon,
  Download as DownloadIcon,
  Visibility as ViewIcon,
  Upload as UploadIcon,
  Launch as LaunchIcon,
  DeleteForever as YankIcon,
} from '@mui/icons-material';
import IconButton from '@mui/material/IconButton';
import Tooltip from '@mui/material/Tooltip';
import { PublishScenarioModal } from '../components/marketplace/PublishScenarioModal';
import { MarketplaceTabs } from '../components/marketplace/MarketplaceTabs';
import { authenticatedFetch } from '../utils/apiClient';
import { useAuthStore } from '../stores/useAuthStore';
import { apiErrorMessage } from '../utils/errorHandling';

interface Scenario {
  name: string;
  description: string;
  author: string;
  author_id: string;
  author_email?: string;
  version: string;
  category: string;
  tags: string[];
  downloads: number;
  rating: number;
  reviews_count: number;
  stars: number;
  repository?: string;
  homepage?: string;
  license: string;
  created_at: string;
  updated_at: string;
  versions: ScenarioVersion[];
}

interface ScenarioVersion {
  version: string;
  download_url: string;
  checksum: string;
  size: number;
  published_at: string;
  yanked: boolean;
  min_mockforge_version?: string;
}

interface Review {
  id: string;
  reviewer: string;
  reviewer_email?: string;
  rating: number;
  title?: string;
  comment: string;
  created_at: string;
  helpful_count: number;
  verified_purchase: boolean;
}

type Snack = { severity: 'success' | 'error' | 'info'; message: string } | null;

export const ScenarioMarketplacePage: React.FC = () => {
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated);
  const currentUserId = useAuthStore((s) => s.user?.id);

  const [scenarios, setScenarios] = useState<Scenario[]>([]);
  const [filteredScenarios, setFilteredScenarios] = useState<Scenario[]>([]);
  const [selectedScenario, setSelectedScenario] = useState<Scenario | null>(null);
  const [reviews, setReviews] = useState<Review[]>([]);
  const [detailsOpen, setDetailsOpen] = useState(false);
  const [publishModalOpen, setPublishModalOpen] = useState(false);
  const [loading, setLoading] = useState(false);
  const [snack, setSnack] = useState<Snack>(null);

  // Review form
  const [reviewRating, setReviewRating] = useState<number>(5);
  const [reviewComment, setReviewComment] = useState('');
  const [reviewTitle, setReviewTitle] = useState('');
  const [reviewSubmitting, setReviewSubmitting] = useState(false);

  // Star state: map of scenario.name → starred bool
  const [starred, setStarred] = useState<Record<string, boolean>>({});
  const [starringName, setStarringName] = useState<string | null>(null);

  // Track which review IDs have been voted in this session, so the UI
  // disables the button after a successful click. (The backend does not
  // track per-user votes, so this is best-effort.)
  const [votedReviews, setVotedReviews] = useState<Record<string, boolean>>({});

  // Filters
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedCategory, setSelectedCategory] = useState('all');
  const [sortBy, setSortBy] = useState('downloads');
  const [minRating, setMinRating] = useState(0);
  const [page, setPage] = useState(0);
  const [perPage] = useState(20);

  const categories = [
    { value: 'all', label: 'All Categories' },
    { value: 'network-chaos', label: 'Network Chaos' },
    { value: 'service-failure', label: 'Service Failure' },
    { value: 'load-testing', label: 'Load Testing' },
    { value: 'resilience-testing', label: 'Resilience Testing' },
    { value: 'security-testing', label: 'Security Testing' },
    { value: 'data-corruption', label: 'Data Corruption' },
    { value: 'multi-protocol', label: 'Multi-Protocol' },
    { value: 'custom-scenario', label: 'Custom Scenario' },
    { value: 'other', label: 'Other' },
  ];

  const sortOptions = [
    { value: 'downloads', label: 'Most Downloaded' },
    { value: 'rating', label: 'Top Rated' },
    { value: 'recent', label: 'Recently Updated' },
    { value: 'name', label: 'Name (A-Z)' },
  ];

  // Load scenarios
  useEffect(() => {
    loadScenarios();
  }, [page, sortBy]);

  // Apply filters
  useEffect(() => {
    filterScenarios();
  }, [scenarios, searchQuery, selectedCategory, sortBy, minRating]);

  const loadScenarios = async () => {
    setLoading(true);
    try {
      const response = await fetch('/api/v1/marketplace/scenarios/search', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          query: searchQuery || null,
          category: selectedCategory !== 'all' ? selectedCategory : null,
          tags: [],
          sort: sortBy,
          page,
          per_page: perPage,
        }),
      });

      if (response.ok) {
        const data = await response.json();
        setScenarios(data.scenarios || []);
      } else {
        const errorData = await response.json().catch(() => ({}));
        setSnack({
          severity: 'error',
          message: apiErrorMessage(response, errorData, 'Failed to load scenarios'),
        });
        setScenarios([]);
      }
    } catch (error) {
      setSnack({
        severity: 'error',
        message: error instanceof Error ? error.message : 'Failed to load scenarios',
      });
      setScenarios([]);
    } finally {
      setLoading(false);
    }
  };

  const filterScenarios = () => {
    let filtered = [...scenarios];

    // Search filter
    if (searchQuery) {
      const query = searchQuery.toLowerCase();
      filtered = filtered.filter(
        (s) =>
          s.name.toLowerCase().includes(query) ||
          s.description.toLowerCase().includes(query) ||
          s.author.toLowerCase().includes(query)
      );
    }

    // Category filter
    if (selectedCategory !== 'all') {
      filtered = filtered.filter((s) => s.category === selectedCategory);
    }

    // Rating filter
    if (minRating > 0) {
      filtered = filtered.filter((s) => s.rating >= minRating);
    }

    setFilteredScenarios(filtered);
  };

  const loadReviews = async (scenarioName: string) => {
    try {
      const response = await fetch(
        `/api/v1/marketplace/scenarios/${encodeURIComponent(scenarioName)}/reviews?page=0&per_page=20`
      );
      if (response.ok) {
        const data = await response.json();
        setReviews(data.reviews || []);
      } else {
        setReviews([]);
      }
    } catch {
      setReviews([]);
    }
  };

  const handleViewDetails = async (scenario: Scenario) => {
    setSelectedScenario(scenario);
    setReviews([]);
    setReviewRating(5);
    setReviewTitle('');
    setReviewComment('');
    setDetailsOpen(true);
    await loadReviews(scenario.name);
  };

  const handleDownload = async (scenarioName: string, version?: string) => {
    try {
      const targetVersion = version || selectedScenario?.version || 'latest';
      const response = await fetch(
        `/api/v1/marketplace/scenarios/${encodeURIComponent(scenarioName)}/versions/${encodeURIComponent(targetVersion)}`
      );

      if (response.ok) {
        const data = await response.json();
        window.open(data.download_url, '_blank');
        setSnack({
          severity: 'success',
          message: `Downloading ${scenarioName}@${targetVersion}`,
        });
        loadScenarios();
      } else {
        const errorData = await response.json().catch(() => ({}));
        setSnack({
          severity: 'error',
          message: apiErrorMessage(response, errorData, 'Failed to download scenario'),
        });
      }
    } catch (error) {
      setSnack({
        severity: 'error',
        message: error instanceof Error ? error.message : 'Failed to download scenario',
      });
    }
  };

  const handleSubmitReview = async () => {
    if (!selectedScenario) return;
    if (reviewComment.trim().length < 10) {
      setSnack({ severity: 'error', message: 'Comment must be at least 10 characters' });
      return;
    }
    setReviewSubmitting(true);
    try {
      const response = await authenticatedFetch(
        `/api/v1/marketplace/scenarios/${encodeURIComponent(selectedScenario.name)}/reviews`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            rating: reviewRating,
            title: reviewTitle.trim() ? reviewTitle.trim() : undefined,
            comment: reviewComment.trim(),
          }),
        }
      );

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        throw new Error(apiErrorMessage(response, errorData, 'Failed to submit review'));
      }

      setSnack({ severity: 'success', message: 'Review submitted' });
      setReviewTitle('');
      setReviewComment('');
      setReviewRating(5);
      await loadReviews(selectedScenario.name);
      // Reload scenarios so updated rating is reflected in the grid
      loadScenarios();
    } catch (error) {
      setSnack({
        severity: 'error',
        message: error instanceof Error ? error.message : 'Failed to submit review',
      });
    } finally {
      setReviewSubmitting(false);
    }
  };

  const handleToggleStar = async (scenario: Scenario, e?: React.MouseEvent) => {
    e?.stopPropagation();
    if (!isAuthenticated) {
      setSnack({ severity: 'info', message: 'Sign in to star scenarios.' });
      return;
    }
    setStarringName(scenario.name);
    try {
      const response = await authenticatedFetch(
        `/api/v1/marketplace/scenarios/${encodeURIComponent(scenario.name)}/star`,
        { method: 'POST' }
      );
      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        throw new Error(apiErrorMessage(response, errorData, 'Failed to toggle star'));
      }
      const data = (await response.json()) as { starred: boolean; stars: number };
      setStarred((prev) => ({ ...prev, [scenario.name]: data.starred }));
      setScenarios((prev) =>
        prev.map((s) =>
          s.name === scenario.name ? { ...s, stars: data.stars } : s
        )
      );
      // Keep the open details dialog in sync if it's the same scenario
      setSelectedScenario((prev) =>
        prev && prev.name === scenario.name ? { ...prev, stars: data.stars } : prev
      );
    } catch (error) {
      setSnack({
        severity: 'error',
        message: error instanceof Error ? error.message : 'Failed to toggle star',
      });
    } finally {
      setStarringName(null);
    }
  };

  const handleYankVersion = async (scenarioName: string, version: string) => {
    const ok = window.confirm(
      `Yank version ${version}? Existing pinned downloads keep working but the version will be hidden from search.`
    );
    if (!ok) return;
    try {
      const response = await authenticatedFetch(
        `/api/v1/marketplace/scenarios/${encodeURIComponent(scenarioName)}/versions/${encodeURIComponent(version)}/yank`,
        { method: 'DELETE' }
      );
      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        throw new Error(apiErrorMessage(response, errorData, 'Failed to yank version'));
      }
      setSnack({ severity: 'success', message: `Version ${version} yanked` });
      // Refresh both list + details so the yanked row drops out of view
      loadScenarios();
      if (selectedScenario && selectedScenario.name === scenarioName) {
        setSelectedScenario({
          ...selectedScenario,
          versions: selectedScenario.versions.map((v) =>
            v.version === version ? { ...v, yanked: true } : v
          ),
        });
      }
    } catch (error) {
      setSnack({
        severity: 'error',
        message: error instanceof Error ? error.message : 'Failed to yank version',
      });
    }
  };

  const handleVoteReview = async (reviewId: string) => {
    if (!selectedScenario) return;
    if (!isAuthenticated) {
      setSnack({ severity: 'info', message: 'Sign in to vote on reviews.' });
      return;
    }
    if (votedReviews[reviewId]) return;
    try {
      const response = await authenticatedFetch(
        `/api/v1/marketplace/scenarios/${encodeURIComponent(selectedScenario.name)}/reviews/${encodeURIComponent(reviewId)}/vote`,
        { method: 'POST' }
      );
      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        throw new Error(apiErrorMessage(response, errorData, 'Failed to record vote'));
      }
      setVotedReviews((prev) => ({ ...prev, [reviewId]: true }));
      setReviews((prev) =>
        prev.map((r) =>
          r.id === reviewId ? { ...r, helpful_count: r.helpful_count + 1 } : r
        )
      );
    } catch (error) {
      setSnack({
        severity: 'error',
        message: error instanceof Error ? error.message : 'Failed to record vote',
      });
    }
  };

  // When authenticated, preload per-scenario starred state for what's on screen.
  useEffect(() => {
    if (!isAuthenticated || scenarios.length === 0) return;
    let cancelled = false;
    (async () => {
      const entries = await Promise.all(
        scenarios.map(async (s) => {
          try {
            const resp = await authenticatedFetch(
              `/api/v1/marketplace/scenarios/${encodeURIComponent(s.name)}/star`
            );
            if (!resp.ok) return [s.name, false] as const;
            const data = (await resp.json()) as { starred: boolean };
            return [s.name, !!data.starred] as const;
          } catch {
            return [s.name, false] as const;
          }
        })
      );
      if (!cancelled) {
        setStarred(Object.fromEntries(entries));
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [scenarios, isAuthenticated]);

  return (
    <Box sx={{ p: 3 }}>
      <MarketplaceTabs />

      {/* Header */}
      <Box sx={{ mb: 4, display: 'flex', justifyContent: 'space-between', alignItems: 'start' }}>
        <Box>
          <Typography variant="h4" gutterBottom>
            Scenario Marketplace
          </Typography>
          <Typography variant="body1" color="text.secondary">
            Browse and discover complete mock configurations and scenarios
          </Typography>
        </Box>
        <Button
          variant="contained"
          startIcon={<UploadIcon />}
          onClick={() => setPublishModalOpen(true)}
        >
          Publish Scenario
        </Button>
      </Box>

      {/* Filters */}
      <Card sx={{ mb: 3 }}>
        <CardContent>
          <Grid container spacing={2} alignItems="center">
            <Grid item xs={12} md={4}>
              <TextField
                fullWidth
                placeholder="Search scenarios..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                onKeyPress={(e) => {
                  if (e.key === 'Enter') {
                    loadScenarios();
                  }
                }}
                InputProps={{
                  startAdornment: <SearchIcon sx={{ mr: 1, color: 'text.secondary' }} />,
                }}
              />
            </Grid>
            <Grid item xs={12} md={3}>
              <FormControl fullWidth>
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
            <Grid item xs={12} md={3}>
              <FormControl fullWidth>
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
            <Grid item xs={12} md={2}>
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                <Typography variant="body2">Min Rating:</Typography>
                <Rating
                  value={minRating}
                  onChange={(_, value) => setMinRating(value || 0)}
                  size="small"
                />
              </Box>
            </Grid>
          </Grid>
        </CardContent>
      </Card>

      {/* Results Summary */}
      <Box sx={{ mb: 2, display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <Typography variant="body2" color="text.secondary">
          {filteredScenarios.length} scenario{filteredScenarios.length !== 1 ? 's' : ''} found
        </Typography>
        <Box sx={{ display: 'flex', gap: 1 }}>
          <Button
            size="small"
            disabled={page === 0}
            onClick={() => setPage((p) => Math.max(0, p - 1))}
          >
            Previous
          </Button>
          <Button size="small" onClick={() => setPage((p) => p + 1)}>
            Next
          </Button>
        </Box>
      </Box>

      {/* Scenarios Grid */}
      {loading ? (
        <Box sx={{ textAlign: 'center', py: 4 }}>
          <Typography>Loading scenarios...</Typography>
        </Box>
      ) : (
        <Grid container spacing={3}>
          {filteredScenarios.map((scenario) => (
            <Grid item xs={12} md={6} lg={4} key={scenario.name}>
              <Card sx={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
                <CardContent sx={{ flexGrow: 1 }}>
                  <Box sx={{ display: 'flex', justifyContent: 'space-between', mb: 2 }}>
                    <Typography variant="h6">{scenario.name}</Typography>
                    <Chip label={scenario.category} size="small" color="primary" />
                  </Box>

                  <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
                    {scenario.description}
                  </Typography>

                  <Box sx={{ mb: 2 }}>
                    {scenario.tags.slice(0, 3).map((tag) => (
                      <Chip
                        key={tag}
                        label={tag}
                        size="small"
                        sx={{ mr: 0.5, mb: 0.5 }}
                        variant="outlined"
                      />
                    ))}
                  </Box>

                  <Box sx={{ display: 'flex', alignItems: 'center', gap: 2, mb: 1 }}>
                    <Rating value={scenario.rating} readOnly size="small" />
                    <Typography variant="caption" color="text.secondary">
                      ({scenario.reviews_count})
                    </Typography>
                  </Box>

                  <Box sx={{ display: 'flex', gap: 2 }}>
                    <Chip
                      icon={<DownloadIcon />}
                      label={scenario.downloads.toLocaleString()}
                      size="small"
                      variant="outlined"
                    />
                    <Chip
                      icon={<StarIcon />}
                      label={(scenario.stars ?? 0).toLocaleString()}
                      size="small"
                      variant="outlined"
                    />
                  </Box>

                  <Typography variant="caption" color="text.secondary" sx={{ mt: 2, display: 'block' }}>
                    by {scenario.author} • v{scenario.version}
                  </Typography>
                </CardContent>

                <CardActions>
                  <Button
                    size="small"
                    startIcon={<ViewIcon />}
                    onClick={() => handleViewDetails(scenario)}
                  >
                    Details
                  </Button>
                  <Button
                    size="small"
                    startIcon={<DownloadIcon />}
                    onClick={() => handleDownload(scenario.name)}
                  >
                    Download
                  </Button>
                  <Tooltip
                    title={
                      isAuthenticated
                        ? starred[scenario.name]
                          ? 'Unstar'
                          : 'Star'
                        : 'Sign in to star'
                    }
                  >
                    <span>
                      <IconButton
                        size="small"
                        onClick={(e) => handleToggleStar(scenario, e)}
                        disabled={starringName === scenario.name}
                        aria-label={starred[scenario.name] ? 'Unstar scenario' : 'Star scenario'}
                      >
                        {starred[scenario.name] ? (
                          <StarIcon fontSize="small" color="warning" />
                        ) : (
                          <StarBorderIcon fontSize="small" />
                        )}
                      </IconButton>
                    </span>
                  </Tooltip>
                </CardActions>
              </Card>
            </Grid>
          ))}
        </Grid>
      )}

      {filteredScenarios.length === 0 && !loading && (
        <Card sx={{ mt: 3 }}>
          <CardContent>
            <Alert severity="info">No scenarios found. Try adjusting your filters.</Alert>
          </CardContent>
        </Card>
      )}

      {/* Scenario Details Dialog */}
      <Dialog
        open={detailsOpen}
        onClose={() => setDetailsOpen(false)}
        maxWidth="md"
        fullWidth
      >
        {selectedScenario && (
          <>
            <DialogTitle>
              <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'start' }}>
                <Box>
                  <Typography variant="h5">{selectedScenario.name}</Typography>
                  <Typography variant="body2" color="text.secondary">
                    by {selectedScenario.author} • v{selectedScenario.version}
                  </Typography>
                </Box>
                <Chip label={selectedScenario.category} color="primary" />
              </Box>
            </DialogTitle>

            <DialogContent>
              <Box sx={{ mb: 3 }}>
                <Typography variant="body1" paragraph>
                  {selectedScenario.description}
                </Typography>

                <Box sx={{ display: 'flex', gap: 1, mb: 2 }}>
                  {selectedScenario.tags.map((tag) => (
                    <Chip key={tag} label={tag} size="small" />
                  ))}
                </Box>

                <Grid container spacing={2} sx={{ mb: 3 }}>
                  <Grid item xs={3}>
                    <Card variant="outlined">
                      <CardContent>
                        <Typography variant="caption" color="text.secondary">
                          Downloads
                        </Typography>
                        <Typography variant="h6">
                          {selectedScenario.downloads.toLocaleString()}
                        </Typography>
                      </CardContent>
                    </Card>
                  </Grid>
                  <Grid item xs={3}>
                    <Card variant="outlined">
                      <CardContent>
                        <Typography variant="caption" color="text.secondary">
                          Stars
                        </Typography>
                        <Typography variant="h6">
                          {(selectedScenario.stars ?? 0).toLocaleString()}
                        </Typography>
                      </CardContent>
                    </Card>
                  </Grid>
                  <Grid item xs={6}>
                    <Card variant="outlined">
                      <CardContent>
                        <Typography variant="caption" color="text.secondary">
                          Rating
                        </Typography>
                        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                          <Rating value={selectedScenario.rating} readOnly precision={0.5} />
                          <Typography variant="body2">
                            ({selectedScenario.reviews_count} reviews)
                          </Typography>
                        </Box>
                      </CardContent>
                    </Card>
                  </Grid>
                </Grid>

                <Alert severity="info" sx={{ mb: 3 }} icon={<LaunchIcon fontSize="small" />}>
                  Need to roll this scenario through dev → test → prod? Use{' '}
                  <Link
                    component="button"
                    type="button"
                    onClick={() => {
                      setDetailsOpen(false);
                      window.location.assign('/workspaces?tab=promotions');
                    }}
                    sx={{ verticalAlign: 'baseline' }}
                  >
                    Workspace Promotions
                  </Link>{' '}
                  to gate the change behind approval.
                </Alert>

                {(selectedScenario.repository ||
                  selectedScenario.homepage ||
                  selectedScenario.license) && (
                  <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 2, mb: 3 }}>
                    {selectedScenario.license && (
                      <Chip
                        label={`License: ${selectedScenario.license}`}
                        size="small"
                        variant="outlined"
                      />
                    )}
                    {selectedScenario.repository && (
                      <Link
                        href={selectedScenario.repository}
                        target="_blank"
                        rel="noopener noreferrer"
                        sx={{ display: 'inline-flex', alignItems: 'center', gap: 0.5 }}
                      >
                        <LaunchIcon fontSize="inherit" /> Repository
                      </Link>
                    )}
                    {selectedScenario.homepage && (
                      <Link
                        href={selectedScenario.homepage}
                        target="_blank"
                        rel="noopener noreferrer"
                        sx={{ display: 'inline-flex', alignItems: 'center', gap: 0.5 }}
                      >
                        <LaunchIcon fontSize="inherit" /> Homepage
                      </Link>
                    )}
                  </Box>
                )}

                {selectedScenario.versions.length > 0 && (
                  <Box sx={{ mb: 2 }}>
                    <Typography variant="h6" gutterBottom>
                      Available Versions
                    </Typography>
                    <List>
                      {selectedScenario.versions
                        .filter((v) => !v.yanked)
                        .map((version) => (
                          <ListItem
                            key={version.version}
                            secondaryAction={
                              <Box sx={{ display: 'flex', gap: 1 }}>
                                <Button
                                  size="small"
                                  onClick={() =>
                                    handleDownload(selectedScenario.name, version.version)
                                  }
                                >
                                  Download
                                </Button>
                                {currentUserId === selectedScenario.author_id && (
                                  <Tooltip title="Yank this version (hides it from search; pinned downloads still resolve)">
                                    <Button
                                      size="small"
                                      color="warning"
                                      startIcon={<YankIcon fontSize="small" />}
                                      onClick={() =>
                                        handleYankVersion(
                                          selectedScenario.name,
                                          version.version
                                        )
                                      }
                                    >
                                      Yank
                                    </Button>
                                  </Tooltip>
                                )}
                              </Box>
                            }
                          >
                            <ListItemText
                              primary={version.version}
                              secondary={`Published ${new Date(version.published_at).toLocaleDateString()}`}
                            />
                          </ListItem>
                        ))}
                    </List>
                  </Box>
                )}
              </Box>

              <Divider sx={{ mb: 2 }} />

              <Typography variant="h6" gutterBottom>
                Reviews
              </Typography>

              {isAuthenticated ? (
                <Card variant="outlined" sx={{ mb: 2 }}>
                  <CardContent>
                    <Typography variant="subtitle2" gutterBottom>
                      Leave a review
                    </Typography>
                    <Box sx={{ display: 'flex', alignItems: 'center', gap: 2, mb: 2 }}>
                      <Typography variant="body2">Your rating:</Typography>
                      <Rating
                        value={reviewRating}
                        onChange={(_, value) => setReviewRating(value || 1)}
                      />
                    </Box>
                    <TextField
                      fullWidth
                      placeholder="Title (optional)"
                      value={reviewTitle}
                      onChange={(e) => setReviewTitle(e.target.value)}
                      sx={{ mb: 2 }}
                      inputProps={{ maxLength: 200 }}
                    />
                    <TextField
                      fullWidth
                      multiline
                      minRows={2}
                      placeholder="Share your experience (min 10 characters)"
                      value={reviewComment}
                      onChange={(e) => setReviewComment(e.target.value)}
                      sx={{ mb: 2 }}
                    />
                    <Button
                      variant="contained"
                      size="small"
                      disabled={reviewSubmitting || reviewComment.trim().length < 10}
                      onClick={handleSubmitReview}
                    >
                      {reviewSubmitting ? 'Submitting…' : 'Submit review'}
                    </Button>
                  </CardContent>
                </Card>
              ) : (
                <Alert severity="info" sx={{ mb: 2 }}>
                  Sign in to leave a review.
                </Alert>
              )}

              {reviews.length === 0 ? (
                <Alert severity="info">No reviews yet</Alert>
              ) : (
                <List>
                  {reviews.map((review) => (
                    <React.Fragment key={review.id}>
                      <ListItem alignItems="flex-start">
                        <Box sx={{ width: '100%' }}>
                          <Box sx={{ display: 'flex', justifyContent: 'space-between', mb: 1 }}>
                            <Box>
                              <Typography variant="subtitle2">{review.reviewer}</Typography>
                              <Rating value={review.rating} readOnly size="small" />
                            </Box>
                            <Typography variant="caption" color="text.secondary">
                              {new Date(review.created_at).toLocaleDateString()}
                            </Typography>
                          </Box>
                          {review.title && (
                            <Typography variant="subtitle2" sx={{ mb: 0.5 }}>
                              {review.title}
                            </Typography>
                          )}
                          <Typography variant="body2">{review.comment}</Typography>
                          <Box
                            sx={{
                              display: 'flex',
                              alignItems: 'center',
                              gap: 0.5,
                              mt: 1,
                            }}
                          >
                            <Tooltip
                              title={
                                isAuthenticated
                                  ? votedReviews[review.id]
                                    ? 'You marked this review as helpful'
                                    : 'Mark this review as helpful'
                                  : 'Sign in to vote'
                              }
                            >
                              <span>
                                <IconButton
                                  size="small"
                                  onClick={() => handleVoteReview(review.id)}
                                  disabled={!isAuthenticated || !!votedReviews[review.id]}
                                  aria-label="Mark review as helpful"
                                >
                                  <ThumbUpOutlinedIcon fontSize="small" />
                                </IconButton>
                              </span>
                            </Tooltip>
                            <Typography variant="caption" color="text.secondary">
                              Helpful ({review.helpful_count})
                            </Typography>
                          </Box>
                        </Box>
                      </ListItem>
                      <Divider component="li" />
                    </React.Fragment>
                  ))}
                </List>
              )}
            </DialogContent>

            <DialogActions>
              <Button onClick={() => setDetailsOpen(false)}>Close</Button>
              <Button
                variant="contained"
                startIcon={<DownloadIcon />}
                onClick={() => {
                  handleDownload(selectedScenario.name);
                  setDetailsOpen(false);
                }}
              >
                Download
              </Button>
            </DialogActions>
          </>
        )}
      </Dialog>

      {/* Publish Scenario Modal */}
      <PublishScenarioModal
        open={publishModalOpen}
        onClose={() => setPublishModalOpen(false)}
        onSuccess={() => {
          loadScenarios();
          setPublishModalOpen(false);
          setSnack({ severity: 'success', message: 'Scenario published successfully' });
        }}
      />

      <Snackbar
        open={!!snack}
        autoHideDuration={4000}
        onClose={() => setSnack(null)}
        anchorOrigin={{ vertical: 'bottom', horizontal: 'center' }}
      >
        {snack ? (
          <Alert severity={snack.severity} onClose={() => setSnack(null)} sx={{ width: '100%' }}>
            {snack.message}
          </Alert>
        ) : undefined}
      </Snackbar>
    </Box>
  );
};

export default ScenarioMarketplacePage;
