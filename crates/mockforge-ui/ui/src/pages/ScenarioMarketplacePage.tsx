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
  IconButton,
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
  Avatar,
} from '@mui/material';
import {
  Search as SearchIcon,
  Star as StarIcon,
  Download as DownloadIcon,
  Visibility as ViewIcon,
  Category as CategoryIcon,
  TrendingUp as TrendingIcon,
  NewReleases as NewIcon,
  FilterList as FilterIcon,
  Upload as UploadIcon,
} from '@mui/icons-material';
import { PublishScenarioModal } from '../components/marketplace/PublishScenarioModal';

interface Scenario {
  name: string;
  description: string;
  author: string;
  author_email?: string;
  version: string;
  category: string;
  tags: string[];
  downloads: number;
  rating: number;
  reviews_count: number;
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

export const ScenarioMarketplacePage: React.FC = () => {
  const [scenarios, setScenarios] = useState<Scenario[]>([]);
  const [filteredScenarios, setFilteredScenarios] = useState<Scenario[]>([]);
  const [selectedScenario, setSelectedScenario] = useState<Scenario | null>(null);
  const [reviews, setReviews] = useState<Review[]>([]);
  const [detailsOpen, setDetailsOpen] = useState(false);
  const [publishModalOpen, setPublishModalOpen] = useState(false);
  const [loading, setLoading] = useState(false);

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
      const response = await fetch('/api/v1/scenarios/search', {
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
        console.error('Failed to load scenarios:', response.statusText);
      }
    } catch (error) {
      console.error('Failed to load scenarios:', error);
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

  const handleViewDetails = async (scenario: Scenario) => {
    setSelectedScenario(scenario);
    setDetailsOpen(true);

    // Load reviews
    try {
      const response = await fetch(`/api/v1/scenarios/${scenario.name}/reviews?page=0&per_page=10`);
      if (response.ok) {
        const data = await response.json();
        setReviews(data.reviews || []);
      }
    } catch (error) {
      console.error('Failed to load reviews:', error);
    }
  };

  const handleDownload = async (scenarioName: string, version?: string) => {
    try {
      const targetVersion = version || selectedScenario?.version || 'latest';
      const response = await fetch(`/api/v1/scenarios/${scenarioName}/versions/${targetVersion}`);

      if (response.ok) {
        const data = await response.json();
        // Trigger download
        window.open(data.download_url, '_blank');
        loadScenarios(); // Refresh to update download count
      }
    } catch (error) {
      console.error('Failed to download scenario:', error);
    }
  };

  return (
    <Box sx={{ p: 3 }}>
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
            onClick={() => setPage((p) => Math.max(0, p - 1)))}
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
                  <Grid item xs={4}>
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
                  <Grid item xs={8}>
                    <Card variant="outlined">
                      <CardContent>
                        <Typography variant="caption" color="text.secondary">
                          Rating
                        </Typography>
                        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                          <Rating value={selectedScenario.rating} readOnly />
                          <Typography variant="body2">
                            ({selectedScenario.reviews_count} reviews)
                          </Typography>
                        </Box>
                      </CardContent>
                    </Card>
                  </Grid>
                </Grid>

                {selectedScenario.versions.length > 0 && (
                  <Box sx={{ mb: 2 }}>
                    <Typography variant="h6" gutterBottom>
                      Available Versions
                    </Typography>
                    <List>
                      {selectedScenario.versions
                        .filter((v) => !v.yanked)
                        .map((version) => (
                          <ListItem key={version.version}>
                            <ListItemText
                              primary={version.version}
                              secondary={`Published ${new Date(version.published_at).toLocaleDateString()}`}
                            />
                            <Button
                              size="small"
                              onClick={() => handleDownload(selectedScenario.name, version.version)}
                            >
                              Download
                            </Button>
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
        }}
      />
    </Box>
  );
};

export default ScenarioMarketplacePage;
