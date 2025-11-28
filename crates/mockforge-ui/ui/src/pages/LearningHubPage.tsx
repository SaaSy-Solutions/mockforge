/**
 * Learning Hub Page
 *
 * Browse tutorials, guides, examples, and learning resources
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
  Divider,
  Alert,
  Paper,
  Accordion,
  AccordionSummary,
  AccordionDetails,
} from '@mui/material';
import {
  Search as SearchIcon,
  School as SchoolIcon,
  Code as CodeIcon,
  VideoLibrary as VideoIcon,
  MenuBook as GuideIcon,
  ExpandMore as ExpandMoreIcon,
  PlayArrow as PlayIcon,
  Visibility as ViewIcon,
} from '@mui/icons-material';
import { communityApi, type LearningResource } from '../services/communityApi';

export const LearningHubPage: React.FC = () => {
  const [resources, setResources] = useState<LearningResource[]>([]);
  const [selectedResource, setSelectedResource] = useState<LearningResource | null>(null);
  const [detailsOpen, setDetailsOpen] = useState(false);
  const [loading, setLoading] = useState(false);
  const [activeTab, setActiveTab] = useState(0);

  // Filters
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedCategory, setSelectedCategory] = useState('all');
  const [selectedType, setSelectedType] = useState('all');
  const [selectedDifficulty, setSelectedDifficulty] = useState('all');
  const [categories, setCategories] = useState<string[]>([]);

  const resourceTypes = [
    { value: 'all', label: 'All Types', icon: <SchoolIcon /> },
    { value: 'tutorial', label: 'Tutorials', icon: <GuideIcon /> },
    { value: 'example', label: 'Examples', icon: <CodeIcon /> },
    { value: 'video', label: 'Videos', icon: <VideoIcon /> },
    { value: 'guide', label: 'Guides', icon: <MenuBook /> },
  ];

  const difficulties = [
    { value: 'all', label: 'All Levels' },
    { value: 'beginner', label: 'Beginner' },
    { value: 'intermediate', label: 'Intermediate' },
    { value: 'advanced', label: 'Advanced' },
  ];

  // Load data
  useEffect(() => {
    loadResources();
    loadCategories();
  }, [selectedCategory, selectedType, selectedDifficulty]);

  const loadResources = async () => {
    setLoading(true);
    try {
      const response = await communityApi.getLearningResources({
        category: selectedCategory !== 'all' ? selectedCategory : undefined,
        type: selectedType !== 'all' ? selectedType : undefined,
        difficulty: selectedDifficulty !== 'all' ? selectedDifficulty : undefined,
        limit: 50,
      });
      if (response.success && response.data) {
        setResources(response.data);
      }
    } catch (error) {
      console.error('Failed to load resources:', error);
    } finally {
      setLoading(false);
    }
  };

  const loadCategories = async () => {
    try {
      const response = await communityApi.getLearningCategories();
      if (response.success && response.data) {
        setCategories(response.data);
      }
    } catch (error) {
      console.error('Failed to load categories:', error);
    }
  };

  const filteredResources = resources.filter((resource) => {
    if (searchQuery) {
      const query = searchQuery.toLowerCase();
      return (
        resource.title.toLowerCase().includes(query) ||
        resource.description.toLowerCase().includes(query) ||
        resource.tags.some((tag) => tag.toLowerCase().includes(query))
      );
    }
    return true;
  });

  const handleViewDetails = async (resource: LearningResource) => {
    try {
      const response = await communityApi.getLearningResource(resource.id);
      if (response.success && response.data) {
        setSelectedResource(response.data);
        setDetailsOpen(true);
      }
    } catch (error) {
      console.error('Failed to load resource details:', error);
    }
  };

  const getResourceIcon = (type: string) => {
    switch (type) {
      case 'tutorial':
        return <GuideIcon />;
      case 'example':
        return <CodeIcon />;
      case 'video':
        return <VideoIcon />;
      default:
        return <SchoolIcon />;
    }
  };

  const getDifficultyColor = (difficulty: string) => {
    switch (difficulty) {
      case 'beginner':
        return 'success';
      case 'intermediate':
        return 'warning';
      case 'advanced':
        return 'error';
      default:
        return 'default';
    }
  };

  return (
    <Box sx={{ p: 3 }}>
      <Typography variant="h4" gutterBottom>
        Learning Hub
      </Typography>
      <Typography variant="body1" color="text.secondary" paragraph>
        Learn MockForge with tutorials, examples, guides, and video resources
      </Typography>

      {/* Search and Filters */}
      <Box sx={{ mb: 3, display: 'flex', gap: 2, flexWrap: 'wrap' }}>
        <TextField
          placeholder="Search resources..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          InputProps={{
            startAdornment: <SearchIcon sx={{ mr: 1, color: 'text.secondary' }} />,
          }}
          sx={{ flexGrow: 1, minWidth: 300 }}
        />
        <FormControl sx={{ minWidth: 150 }}>
          <InputLabel>Type</InputLabel>
          <Select
            value={selectedType}
            onChange={(e) => setSelectedType(e.target.value)}
            label="Type"
          >
            {resourceTypes.map((type) => (
              <MenuItem key={type.value} value={type.value}>
                {type.label}
              </MenuItem>
            ))}
          </Select>
        </FormControl>
        <FormControl sx={{ minWidth: 150 }}>
          <InputLabel>Category</InputLabel>
          <Select
            value={selectedCategory}
            onChange={(e) => setSelectedCategory(e.target.value)}
            label="Category"
          >
            <MenuItem value="all">All Categories</MenuItem>
            {categories.map((cat) => (
              <MenuItem key={cat} value={cat}>
                {cat.charAt(0).toUpperCase() + cat.slice(1).replace(/-/g, ' ')}
              </MenuItem>
            ))}
          </Select>
        </FormControl>
        <FormControl sx={{ minWidth: 150 }}>
          <InputLabel>Difficulty</InputLabel>
          <Select
            value={selectedDifficulty}
            onChange={(e) => setSelectedDifficulty(e.target.value)}
            label="Difficulty"
          >
            {difficulties.map((diff) => (
              <MenuItem key={diff.value} value={diff.value}>
                {diff.label}
              </MenuItem>
            ))}
          </Select>
        </FormControl>
      </Box>

      {/* Resources Grid */}
      {loading ? (
        <Typography>Loading resources...</Typography>
      ) : filteredResources.length === 0 ? (
        <Alert severity="info">No resources found. Try adjusting your filters.</Alert>
      ) : (
        <Grid container spacing={3}>
          {filteredResources.map((resource) => (
            <Grid item xs={12} sm={6} md={4} key={resource.id}>
              <Card sx={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
                <CardContent sx={{ flexGrow: 1 }}>
                  <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 1 }}>
                    {getResourceIcon(resource.resource_type)}
                    <Typography variant="h6" component="div" sx={{ flexGrow: 1 }}>
                      {resource.title}
                    </Typography>
                  </Box>
                  <Typography variant="body2" color="text.secondary" paragraph>
                    {resource.description}
                  </Typography>
                  <Box sx={{ display: 'flex', gap: 1, flexWrap: 'wrap', mb: 2 }}>
                    <Chip
                      label={resource.difficulty}
                      size="small"
                      color={getDifficultyColor(resource.difficulty) as any}
                    />
                    {resource.tags.slice(0, 2).map((tag) => (
                      <Chip key={tag} label={tag} size="small" variant="outlined" />
                    ))}
                  </Box>
                  <Box sx={{ display: 'flex', alignItems: 'center', gap: 2 }}>
                    <Box sx={{ display: 'flex', alignItems: 'center' }}>
                      <Rating value={resource.rating} readOnly size="small" />
                      <Typography variant="caption" sx={{ ml: 1 }}>
                        {resource.rating.toFixed(1)}
                      </Typography>
                    </Box>
                    <Typography variant="caption" color="text.secondary">
                      {resource.views} views
                    </Typography>
                  </Box>
                </CardContent>
                <CardActions>
                  <Button size="small" onClick={() => handleViewDetails(resource)}>
                    View Details
                  </Button>
                  {resource.content_url && (
                    <Button
                      size="small"
                      startIcon={<ViewIcon />}
                      href={resource.content_url}
                      target="_blank"
                    >
                      Read
                    </Button>
                  )}
                  {resource.video_url && (
                    <Button
                      size="small"
                      startIcon={<PlayIcon />}
                      href={resource.video_url}
                      target="_blank"
                    >
                      Watch
                    </Button>
                  )}
                </CardActions>
              </Card>
            </Grid>
          ))}
        </Grid>
      )}

      {/* Resource Details Dialog */}
      <Dialog open={detailsOpen} onClose={() => setDetailsOpen(false)} maxWidth="md" fullWidth>
        {selectedResource && (
          <>
            <DialogTitle>
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                {getResourceIcon(selectedResource.resource_type)}
                {selectedResource.title}
              </Box>
            </DialogTitle>
            <DialogContent>
              <Box sx={{ mb: 2 }}>
                <Typography variant="body1" paragraph>
                  {selectedResource.description}
                </Typography>
                <Box sx={{ display: 'flex', gap: 1, flexWrap: 'wrap', mb: 2 }}>
                  <Chip
                    label={selectedResource.difficulty}
                    size="small"
                    color={getDifficultyColor(selectedResource.difficulty) as any}
                  />
                  <Chip label={selectedResource.resource_type} size="small" variant="outlined" />
                  {selectedResource.tags.map((tag) => (
                    <Chip key={tag} label={tag} size="small" variant="outlined" />
                  ))}
                </Box>
                {selectedResource.code_examples.length > 0 && (
                  <Box sx={{ mb: 2 }}>
                    <Typography variant="subtitle2" gutterBottom>
                      Code Examples
                    </Typography>
                    {selectedResource.code_examples.map((example, idx) => (
                      <Accordion key={idx}>
                        <AccordionSummary expandIcon={<ExpandMoreIcon />}>
                          <Typography variant="subtitle2">
                            {example.title} ({example.language})
                          </Typography>
                        </AccordionSummary>
                        <AccordionDetails>
                          {example.description && (
                            <Typography variant="body2" paragraph>
                              {example.description}
                            </Typography>
                          )}
                          <Paper
                            component="pre"
                            sx={{
                              p: 2,
                              bgcolor: 'grey.100',
                              overflow: 'auto',
                              fontSize: '0.875rem',
                            }}
                          >
                            <code>{example.code}</code>
                          </Paper>
                        </AccordionDetails>
                      </Accordion>
                    ))}
                  </Box>
                )}
                <Box sx={{ display: 'flex', gap: 3 }}>
                  <Box>
                    <Typography variant="caption" color="text.secondary">
                      Views
                    </Typography>
                    <Typography variant="h6">{selectedResource.views}</Typography>
                  </Box>
                  <Box>
                    <Typography variant="caption" color="text.secondary">
                      Rating
                    </Typography>
                    <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                      <Rating value={selectedResource.rating} readOnly size="small" />
                      <Typography variant="body2">{selectedResource.rating.toFixed(1)}</Typography>
                    </Box>
                  </Box>
                  <Box>
                    <Typography variant="caption" color="text.secondary">
                      Author
                    </Typography>
                    <Typography variant="body2">{selectedResource.author}</Typography>
                  </Box>
                </Box>
              </Box>
            </DialogContent>
            <DialogActions>
              {selectedResource.content_url && (
                <Button href={selectedResource.content_url} target="_blank" startIcon={<ViewIcon />}>
                  Read Full Guide
                </Button>
              )}
              {selectedResource.video_url && (
                <Button href={selectedResource.video_url} target="_blank" startIcon={<PlayIcon />}>
                  Watch Video
                </Button>
              )}
              <Button onClick={() => setDetailsOpen(false)}>Close</Button>
            </DialogActions>
          </>
        )}
      </Dialog>
    </Box>
  );
};
