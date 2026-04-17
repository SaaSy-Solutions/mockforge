/**
 * Template Marketplace Page
 *
 * Browse, search, and install orchestration templates from the marketplace.
 */

import React, { useState, useEffect, useCallback } from 'react';
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
  Divider,
  Alert,
  Snackbar,
  CircularProgress,
} from '@mui/material';
import {
  Search as SearchIcon,
  Star as StarIcon,
  StarBorder as StarBorderIcon,
  Download as DownloadIcon,
  Visibility as ViewIcon,
} from '@mui/icons-material';
import IconButton from '@mui/material/IconButton';
import Tooltip from '@mui/material/Tooltip';
import { authenticatedFetch } from '../utils/apiClient';
import { useAuthStore } from '../stores/useAuthStore';
import { MarketplaceTabs } from '../components/marketplace/MarketplaceTabs';

interface TemplateStats {
  downloads: number;
  stars: number;
  forks: number;
  rating: number;
  rating_count: number;
}

interface Template {
  id: string;
  name: string;
  description: string;
  author: string;
  version: string;
  category: string;
  tags: string[];
  stats: TemplateStats;
  created_at: string;
  updated_at: string;
  content?: unknown;
}

interface Review {
  id: string;
  reviewer: string;
  rating: number;
  title?: string | null;
  comment: string;
  created_at: string;
  helpful_count: number;
  verified_use?: boolean;
}

type Snack = { severity: 'success' | 'error' | 'info'; message: string } | null;

export const TemplateMarketplacePage: React.FC = () => {
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated);

  const [templates, setTemplates] = useState<Template[]>([]);
  const [filteredTemplates, setFilteredTemplates] = useState<Template[]>([]);
  const [selectedTemplate, setSelectedTemplate] = useState<Template | null>(null);
  const [reviews, setReviews] = useState<Review[]>([]);
  const [detailsOpen, setDetailsOpen] = useState(false);
  const [loading, setLoading] = useState(false);
  const [snack, setSnack] = useState<Snack>(null);

  // Filters
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedCategory, setSelectedCategory] = useState('all');
  const [sortBy, setSortBy] = useState('popular');
  const [minRating, setMinRating] = useState(0);

  // Review form
  const [reviewRating, setReviewRating] = useState<number>(5);
  const [reviewComment, setReviewComment] = useState('');
  const [reviewSubmitting, setReviewSubmitting] = useState(false);

  // Star state: map of `${name}@${version}` → starred bool
  const [starred, setStarred] = useState<Record<string, boolean>>({});
  const [starringKey, setStarringKey] = useState<string | null>(null);

  const categories = [
    { value: 'all', label: 'All Categories' },
    { value: 'network-chaos', label: 'Network Chaos' },
    { value: 'service-failure', label: 'Service Failure' },
    { value: 'load-testing', label: 'Load Testing' },
    { value: 'resilience-testing', label: 'Resilience Testing' },
    { value: 'security-testing', label: 'Security Testing' },
    { value: 'multi-protocol', label: 'Multi-Protocol' },
  ];

  const sortOptions = [
    { value: 'popular', label: 'Most Popular' },
    { value: 'newest', label: 'Newest First' },
    { value: 'top-rated', label: 'Top Rated' },
    { value: 'most-downloaded', label: 'Most Downloaded' },
  ];

  const loadTemplates = useCallback(async () => {
    setLoading(true);
    try {
      const body: Record<string, unknown> = {
        tags: [],
        page: 0,
        per_page: 100,
      };
      if (searchQuery.trim()) body.query = searchQuery.trim();
      if (selectedCategory !== 'all') body.category = selectedCategory;

      const response = await fetch('/api/v1/marketplace/templates/search', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      });

      if (!response.ok) {
        throw new Error(`Search failed: ${response.status}`);
      }

      const data = await response.json();
      setTemplates(Array.isArray(data.templates) ? data.templates : []);
    } catch (error) {
      console.error('Failed to load templates:', error);
      setSnack({ severity: 'error', message: 'Failed to load templates' });
      setTemplates([]);
    } finally {
      setLoading(false);
    }
  }, [searchQuery, selectedCategory]);

  // Initial load + reload when server-side filters change
  useEffect(() => {
    loadTemplates();
  }, [loadTemplates]);

  // Client-side filtering (rating) + sorting
  useEffect(() => {
    let filtered = [...templates];

    if (minRating > 0) {
      filtered = filtered.filter((t) => t.stats.rating >= minRating);
    }

    switch (sortBy) {
      case 'popular':
        filtered.sort((a, b) => {
          const scoreA = a.stats.downloads + a.stats.stars * 2;
          const scoreB = b.stats.downloads + b.stats.stars * 2;
          return scoreB - scoreA;
        });
        break;
      case 'newest':
        filtered.sort(
          (a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime()
        );
        break;
      case 'top-rated':
        filtered.sort((a, b) => b.stats.rating - a.stats.rating);
        break;
      case 'most-downloaded':
        filtered.sort((a, b) => b.stats.downloads - a.stats.downloads);
        break;
    }

    setFilteredTemplates(filtered);
  }, [templates, sortBy, minRating]);

  const starKey = (t: Template) => `${t.name}@${t.version}`;

  const handleToggleStar = async (template: Template, e?: React.MouseEvent) => {
    e?.stopPropagation();
    if (!isAuthenticated) {
      setSnack({ severity: 'info', message: 'Sign in to star templates.' });
      return;
    }
    const key = starKey(template);
    setStarringKey(key);
    try {
      const resp = await authenticatedFetch(
        `/api/v1/marketplace/templates/${encodeURIComponent(template.name)}/${encodeURIComponent(template.version)}/star`,
        { method: 'POST' }
      );
      if (!resp.ok) {
        const err = await resp.json().catch(() => ({}));
        throw new Error(err.error ?? err.message ?? `HTTP ${resp.status}`);
      }
      const data = (await resp.json()) as { starred: boolean; stars: number };
      setStarred((prev) => ({ ...prev, [key]: data.starred }));
      setTemplates((prev) =>
        prev.map((t) =>
          t.name === template.name && t.version === template.version
            ? { ...t, stats: { ...t.stats, stars: data.stars } }
            : t
        )
      );
    } catch (error) {
      const msg = error instanceof Error ? error.message : 'Failed to toggle star';
      setSnack({ severity: 'error', message: msg });
    } finally {
      setStarringKey(null);
    }
  };

  // When authenticated, preload per-template starred state for what's on screen.
  useEffect(() => {
    if (!isAuthenticated || templates.length === 0) return;
    let cancelled = false;
    (async () => {
      const entries = await Promise.all(
        templates.map(async (t) => {
          const key = starKey(t);
          try {
            const resp = await authenticatedFetch(
              `/api/v1/marketplace/templates/${encodeURIComponent(t.name)}/${encodeURIComponent(t.version)}/star`
            );
            if (!resp.ok) return [key, false] as const;
            const data = (await resp.json()) as { starred: boolean };
            return [key, !!data.starred] as const;
          } catch {
            return [key, false] as const;
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
  }, [templates, isAuthenticated]);

  const loadReviews = useCallback(async (template: Template) => {
    try {
      const response = await fetch(
        `/api/v1/marketplace/templates/${encodeURIComponent(template.name)}/${encodeURIComponent(template.version)}/reviews?page=0&per_page=50`
      );
      if (response.ok) {
        const data = await response.json();
        setReviews(Array.isArray(data.reviews) ? data.reviews : []);
      } else {
        setReviews([]);
      }
    } catch (error) {
      console.error('Failed to load reviews:', error);
      setReviews([]);
    }
  }, []);

  const handleViewDetails = async (template: Template) => {
    setSelectedTemplate(template);
    setReviews([]);
    setReviewRating(5);
    setReviewComment('');
    setDetailsOpen(true);
    await loadReviews(template);
  };

  const handleDownload = async (template: Template) => {
    try {
      const response = await fetch(
        `/api/v1/marketplace/templates/${encodeURIComponent(template.name)}/${encodeURIComponent(template.version)}`
      );
      if (!response.ok) {
        throw new Error(`Download failed: ${response.status}`);
      }
      const data = await response.json();
      const payload = data.content ?? data;
      const blob = new Blob([JSON.stringify(payload, null, 2)], {
        type: 'application/json',
      });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `${template.name}-${template.version}.json`;
      document.body.appendChild(a);
      a.click();
      a.remove();
      URL.revokeObjectURL(url);
      setSnack({ severity: 'success', message: `Downloaded ${template.name}@${template.version}` });
    } catch (error) {
      console.error('Failed to download template:', error);
      setSnack({ severity: 'error', message: 'Download failed' });
    }
  };

  const handleSubmitReview = async () => {
    if (!selectedTemplate) return;
    if (reviewComment.trim().length < 10) {
      setSnack({ severity: 'error', message: 'Comment must be at least 10 characters' });
      return;
    }
    setReviewSubmitting(true);
    try {
      const response = await authenticatedFetch(
        `/api/v1/marketplace/templates/${encodeURIComponent(selectedTemplate.name)}/${encodeURIComponent(selectedTemplate.version)}/reviews`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            rating: reviewRating,
            comment: reviewComment.trim(),
          }),
        }
      );

      if (!response.ok) {
        const err = await response.json().catch(() => ({}));
        throw new Error(err.error ?? err.message ?? `HTTP ${response.status}`);
      }

      setSnack({ severity: 'success', message: 'Review submitted' });
      setReviewComment('');
      setReviewRating(5);
      await loadReviews(selectedTemplate);
      // Reload templates so updated rating is reflected in the grid
      loadTemplates();
    } catch (error) {
      const msg = error instanceof Error ? error.message : 'Failed to submit review';
      setSnack({ severity: 'error', message: msg });
    } finally {
      setReviewSubmitting(false);
    }
  };

  return (
    <Box sx={{ p: 3 }}>
      <MarketplaceTabs />

      {/* Header */}
      <Box sx={{ mb: 4 }}>
        <Typography variant="h4" gutterBottom>
          Template Marketplace
        </Typography>
        <Typography variant="body1" color="text.secondary">
          Browse and discover chaos orchestration templates
        </Typography>
      </Box>

      {/* Filters */}
      <Card sx={{ mb: 3 }}>
        <CardContent>
          <Grid container spacing={2} alignItems="center">
            <Grid item xs={12} md={4}>
              <TextField
                fullWidth
                placeholder="Search templates..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
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
                />
              </Box>
            </Grid>
          </Grid>
        </CardContent>
      </Card>

      {/* Results Summary */}
      <Box sx={{ mb: 2, display: 'flex', alignItems: 'center', gap: 2 }}>
        <Typography variant="body2" color="text.secondary">
          {filteredTemplates.length} template{filteredTemplates.length !== 1 ? 's' : ''} found
        </Typography>
        {loading && <CircularProgress size={16} />}
      </Box>

      {/* Templates Grid */}
      {!loading && filteredTemplates.length === 0 ? (
        <Alert severity="info">
          No templates match your filters. Try broadening your search.
        </Alert>
      ) : (
        <Grid container spacing={3}>
          {filteredTemplates.map((template) => (
            <Grid item xs={12} md={6} lg={4} key={template.id}>
              <Card sx={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
                <CardContent sx={{ flexGrow: 1 }}>
                  <Box sx={{ display: 'flex', justifyContent: 'space-between', mb: 2 }}>
                    <Typography variant="h6">{template.name}</Typography>
                    <Chip label={template.category} size="small" color="primary" />
                  </Box>

                  <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
                    {template.description}
                  </Typography>

                  <Box sx={{ mb: 2 }}>
                    {template.tags.slice(0, 3).map((tag) => (
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
                    <Rating value={template.stats.rating} readOnly size="small" precision={0.5} />
                    <Typography variant="caption" color="text.secondary">
                      ({template.stats.rating_count})
                    </Typography>
                  </Box>

                  <Box sx={{ display: 'flex', gap: 2 }}>
                    <Chip
                      icon={<DownloadIcon />}
                      label={template.stats.downloads.toLocaleString()}
                      size="small"
                      variant="outlined"
                    />
                    <Chip
                      icon={<StarIcon />}
                      label={template.stats.stars.toLocaleString()}
                      size="small"
                      variant="outlined"
                    />
                  </Box>

                  <Typography variant="caption" color="text.secondary" sx={{ mt: 2, display: 'block' }}>
                    by {template.author} • v{template.version}
                  </Typography>
                </CardContent>

                <CardActions>
                  <Button
                    size="small"
                    startIcon={<ViewIcon />}
                    onClick={() => handleViewDetails(template)}
                  >
                    Details
                  </Button>
                  <Button
                    size="small"
                    startIcon={<DownloadIcon />}
                    onClick={() => handleDownload(template)}
                  >
                    Download
                  </Button>
                  <Tooltip
                    title={
                      isAuthenticated
                        ? starred[starKey(template)]
                          ? 'Unstar'
                          : 'Star'
                        : 'Sign in to star'
                    }
                  >
                    <span>
                      <IconButton
                        size="small"
                        onClick={(e) => handleToggleStar(template, e)}
                        disabled={starringKey === starKey(template)}
                        aria-label={starred[starKey(template)] ? 'Unstar template' : 'Star template'}
                      >
                        {starred[starKey(template)] ? (
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

      {/* Template Details Dialog */}
      <Dialog
        open={detailsOpen}
        onClose={() => setDetailsOpen(false)}
        maxWidth="md"
        fullWidth
      >
        {selectedTemplate && (
          <>
            <DialogTitle>
              <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'start' }}>
                <Box>
                  <Typography variant="h5">{selectedTemplate.name}</Typography>
                  <Typography variant="body2" color="text.secondary">
                    by {selectedTemplate.author} • v{selectedTemplate.version}
                  </Typography>
                </Box>
                <Chip label={selectedTemplate.category} color="primary" />
              </Box>
            </DialogTitle>

            <DialogContent>
              <Box sx={{ mb: 3 }}>
                <Typography variant="body1" paragraph>
                  {selectedTemplate.description}
                </Typography>

                <Box sx={{ display: 'flex', gap: 1, mb: 2, flexWrap: 'wrap' }}>
                  {selectedTemplate.tags.map((tag) => (
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
                          {selectedTemplate.stats.downloads.toLocaleString()}
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
                          {selectedTemplate.stats.stars.toLocaleString()}
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
                          <Rating value={selectedTemplate.stats.rating} readOnly precision={0.5} />
                          <Typography variant="body2">
                            ({selectedTemplate.stats.rating_count} reviews)
                          </Typography>
                        </Box>
                      </CardContent>
                    </Card>
                  </Grid>
                </Grid>
              </Box>

              <Divider sx={{ mb: 2 }} />

              <Typography variant="h6" gutterBottom>
                Reviews
              </Typography>

              {/* Submit-review form */}
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
                          {review.title ? (
                            <Typography variant="body2" sx={{ fontWeight: 500 }}>
                              {review.title}
                            </Typography>
                          ) : null}
                          <Typography variant="body2">{review.comment}</Typography>
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
                  if (selectedTemplate) handleDownload(selectedTemplate);
                  setDetailsOpen(false);
                }}
              >
                Download
              </Button>
            </DialogActions>
          </>
        )}
      </Dialog>

      <Snackbar
        open={!!snack}
        autoHideDuration={4000}
        onClose={() => setSnack(null)}
        anchorOrigin={{ vertical: 'bottom', horizontal: 'center' }}
      >
        {snack ? (
          <Alert
            severity={snack.severity}
            onClose={() => setSnack(null)}
            sx={{ width: '100%' }}
          >
            {snack.message}
          </Alert>
        ) : undefined}
      </Snackbar>
    </Box>
  );
};

export default TemplateMarketplacePage;
