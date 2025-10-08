/**
 * Template Marketplace Page
 *
 * Browse, search, and install orchestration templates from the marketplace.
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
} from '@mui/icons-material';

interface Template {
  id: string;
  name: string;
  description: string;
  author: string;
  version: string;
  category: string;
  tags: string[];
  stats: {
    downloads: number;
    stars: number;
    rating: number;
    ratingCount: number;
  };
  createdAt: Date;
  updatedAt: Date;
}

interface Review {
  id: string;
  userName: string;
  rating: number;
  comment: string;
  createdAt: Date;
  helpfulCount: number;
}

export const TemplateMarketplacePage: React.FC = () => {
  const [templates, setTemplates] = useState<Template[]>([]);
  const [filteredTemplates, setFilteredTemplates] = useState<Template[]>([]);
  const [selectedTemplate, setSelectedTemplate] = useState<Template | null>(null);
  const [reviews, setReviews] = useState<Review[]>([]);
  const [detailsOpen, setDetailsOpen] = useState(false);

  // Filters
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedCategory, setSelectedCategory] = useState('all');
  const [sortBy, setSortBy] = useState('popular');
  const [minRating, setMinRating] = useState(0);

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

  // Load templates
  useEffect(() => {
    loadTemplates();
  }, []);

  // Apply filters
  useEffect(() => {
    filterTemplates();
  }, [templates, searchQuery, selectedCategory, sortBy, minRating]);

  const loadTemplates = async () => {
    try {
      const response = await fetch('/api/chaos/templates/search', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          sortBy,
          limit: 100,
          offset: 0,
        }),
      });

      if (response.ok) {
        const data = await response.json();
        setTemplates(data.templates || []);
      }
    } catch (error) {
      console.error('Failed to load templates:', error);
    }
  };

  const filterTemplates = () => {
    let filtered = [...templates];

    // Search filter
    if (searchQuery) {
      const query = searchQuery.toLowerCase();
      filtered = filtered.filter(
        (t) =>
          t.name.toLowerCase().includes(query) ||
          t.description.toLowerCase().includes(query) ||
          t.author.toLowerCase().includes(query)
      );
    }

    // Category filter
    if (selectedCategory !== 'all') {
      filtered = filtered.filter((t) => t.category === selectedCategory);
    }

    // Rating filter
    if (minRating > 0) {
      filtered = filtered.filter((t) => t.stats.rating >= minRating);
    }

    // Sort
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
          (a, b) => new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime()
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
  };

  const handleViewDetails = async (template: Template) => {
    setSelectedTemplate(template);
    setDetailsOpen(true);

    // Load reviews
    try {
      const response = await fetch(`/api/chaos/templates/${template.id}/reviews`);
      if (response.ok) {
        const data = await response.json();
        setReviews(data.reviews || []);
      }
    } catch (error) {
      console.error('Failed to load reviews:', error);
    }
  };

  const handleDownload = async (templateId: string) => {
    try {
      const response = await fetch(`/api/chaos/templates/${templateId}/download`, {
        method: 'POST',
      });

      if (response.ok) {
        const data = await response.json();
        // Handle successful download
        alert('Template downloaded successfully!');
        loadTemplates(); // Refresh to update download count
      }
    } catch (error) {
      console.error('Failed to download template:', error);
    }
  };

  const handleStar = async (templateId: string) => {
    try {
      await fetch(`/api/chaos/templates/${templateId}/star`, {
        method: 'POST',
      });
      loadTemplates(); // Refresh to update star count
    } catch (error) {
      console.error('Failed to star template:', error);
    }
  };

  return (
    <Box sx={{ p: 3 }}>
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
      <Box sx={{ mb: 2 }}>
        <Typography variant="body2" color="text.secondary">
          {filteredTemplates.length} template{filteredTemplates.length !== 1 ? 's' : ''} found
        </Typography>
      </Box>

      {/* Templates Grid */}
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
                  <Rating value={template.stats.rating} readOnly size="small" />
                  <Typography variant="caption" color="text.secondary">
                    ({template.stats.ratingCount})
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
                  onClick={() => handleDownload(template.id)}
                >
                  Download
                </Button>
                <IconButton size="small" onClick={() => handleStar(template.id)}>
                  <StarIcon />
                </IconButton>
              </CardActions>
            </Card>
          </Grid>
        ))}
      </Grid>

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

                <Box sx={{ display: 'flex', gap: 1, mb: 2 }}>
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
                          <Rating value={selectedTemplate.stats.rating} readOnly />
                          <Typography variant="body2">
                            ({selectedTemplate.stats.ratingCount} reviews)
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
                              <Typography variant="subtitle2">{review.userName}</Typography>
                              <Rating value={review.rating} readOnly size="small" />
                            </Box>
                            <Typography variant="caption" color="text.secondary">
                              {new Date(review.createdAt).toLocaleDateString()}
                            </Typography>
                          </Box>
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
                  handleDownload(selectedTemplate.id);
                  setDetailsOpen(false);
                }}
              >
                Download
              </Button>
            </DialogActions>
          </>
        )}
      </Dialog>
    </Box>
  );
};

export default TemplateMarketplacePage;
