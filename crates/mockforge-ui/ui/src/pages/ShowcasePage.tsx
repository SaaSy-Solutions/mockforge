/**
 * Showcase Gallery Page
 *
 * Browse featured community projects and success stories
 */

import React, { useState, useEffect } from 'react';
import {
  Box,
  Card,
  CardContent,
  CardMedia,
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
  Avatar,
  Divider,
  Alert,
} from '@mui/material';
import {
  Search as SearchIcon,
  Star as StarIcon,
  Download as DownloadIcon,
  Visibility as ViewIcon,
  OpenInNew as OpenIcon,
  Code as CodeIcon,
  TrendingUp as TrendingIcon,
  NewReleases as NewIcon,
} from '@mui/icons-material';
import { communityApi, type ShowcaseProject, type SuccessStory } from '../services/communityApi';

export const ShowcasePage: React.FC = () => {
  const [projects, setProjects] = useState<ShowcaseProject[]>([]);
  const [stories, setStories] = useState<SuccessStory[]>([]);
  const [selectedProject, setSelectedProject] = useState<ShowcaseProject | null>(null);
  const [detailsOpen, setDetailsOpen] = useState(false);
  const [loading, setLoading] = useState(false);
  const [activeTab, setActiveTab] = useState(0);

  // Filters
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedCategory, setSelectedCategory] = useState('all');
  const [showFeaturedOnly, setShowFeaturedOnly] = useState(false);
  const [categories, setCategories] = useState<string[]>([]);

  // Load data
  useEffect(() => {
    loadProjects();
    loadStories();
    loadCategories();
  }, [selectedCategory, showFeaturedOnly]);

  const loadProjects = async () => {
    setLoading(true);
    try {
      const response = await communityApi.getShowcaseProjects({
        category: selectedCategory !== 'all' ? selectedCategory : undefined,
        featured: showFeaturedOnly || undefined,
        limit: 50,
      });
      if (response.success && response.data) {
        setProjects(response.data);
      }
    } catch (error) {
      console.error('Failed to load projects:', error);
    } finally {
      setLoading(false);
    }
  };

  const loadStories = async () => {
    try {
      const response = await communityApi.getSuccessStories({
        featured: true,
        limit: 10,
      });
      if (response.success && response.data) {
        setStories(response.data);
      }
    } catch (error) {
      console.error('Failed to load stories:', error);
    }
  };

  const loadCategories = async () => {
    try {
      const response = await communityApi.getShowcaseCategories();
      if (response.success && response.data) {
        setCategories(response.data);
      }
    } catch (error) {
      console.error('Failed to load categories:', error);
    }
  };

  const filteredProjects = projects.filter((project) => {
    if (searchQuery) {
      const query = searchQuery.toLowerCase();
      return (
        project.title.toLowerCase().includes(query) ||
        project.description.toLowerCase().includes(query) ||
        project.author.toLowerCase().includes(query) ||
        project.tags.some((tag) => tag.toLowerCase().includes(query))
      );
    }
    return true;
  });

  const handleViewDetails = async (project: ShowcaseProject) => {
    try {
      const response = await communityApi.getShowcaseProject(project.id);
      if (response.success && response.data) {
        setSelectedProject(response.data);
        setDetailsOpen(true);
      }
    } catch (error) {
      console.error('Failed to load project details:', error);
    }
  };

  return (
    <Box sx={{ p: 3 }}>
      <Typography variant="h4" gutterBottom>
        Community Showcase
      </Typography>
      <Typography variant="body1" color="text.secondary" paragraph>
        Discover amazing projects built with MockForge and learn from real-world success stories
      </Typography>

      <Tabs value={activeTab} onChange={(_, v) => setActiveTab(v)} sx={{ mb: 3 }}>
        <Tab label="Featured Projects" />
        <Tab label="Success Stories" />
      </Tabs>

      {activeTab === 0 && (
        <>
          {/* Search and Filters */}
          <Box sx={{ mb: 3, display: 'flex', gap: 2, flexWrap: 'wrap' }}>
            <TextField
              placeholder="Search projects..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              InputProps={{
                startAdornment: <SearchIcon sx={{ mr: 1, color: 'text.secondary' }} />,
              }}
              sx={{ flexGrow: 1, minWidth: 300 }}
            />
            <FormControl sx={{ minWidth: 200 }}>
              <InputLabel>Category</InputLabel>
              <Select
                value={selectedCategory}
                onChange={(e) => setSelectedCategory(e.target.value)}
                label="Category"
              >
                <MenuItem value="all">All Categories</MenuItem>
                {categories.map((cat) => (
                  <MenuItem key={cat} value={cat}>
                    {cat.charAt(0).toUpperCase() + cat.slice(1)}
                  </MenuItem>
                ))}
              </Select>
            </FormControl>
            <Button
              variant={showFeaturedOnly ? 'contained' : 'outlined'}
              onClick={() => setShowFeaturedOnly(!showFeaturedOnly)}
              startIcon={<StarIcon />}
            >
              Featured Only
            </Button>
          </Box>

          {/* Projects Grid */}
          {loading ? (
            <Typography>Loading projects...</Typography>
          ) : filteredProjects.length === 0 ? (
            <Alert severity="info">No projects found. Try adjusting your filters.</Alert>
          ) : (
            <Grid container spacing={3}>
              {filteredProjects.map((project) => (
                <Grid item xs={12} sm={6} md={4} key={project.id}>
                  <Card sx={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
                    {project.screenshot && (
                      <CardMedia
                        component="img"
                        height="200"
                        image={project.screenshot}
                        alt={project.title}
                      />
                    )}
                    <CardContent sx={{ flexGrow: 1 }}>
                      <Box sx={{ display: 'flex', alignItems: 'center', mb: 1 }}>
                        <Typography variant="h6" component="div" sx={{ flexGrow: 1 }}>
                          {project.title}
                        </Typography>
                        {project.featured && (
                          <Chip
                            icon={<StarIcon />}
                            label="Featured"
                            size="small"
                            color="primary"
                          />
                        )}
                      </Box>
                      <Typography variant="body2" color="text.secondary" paragraph>
                        {project.description}
                      </Typography>
                      <Box sx={{ display: 'flex', gap: 1, flexWrap: 'wrap', mb: 2 }}>
                        {project.tags.slice(0, 3).map((tag) => (
                          <Chip key={tag} label={tag} size="small" variant="outlined" />
                        ))}
                      </Box>
                      <Box sx={{ display: 'flex', alignItems: 'center', gap: 2 }}>
                        <Box sx={{ display: 'flex', alignItems: 'center' }}>
                          <Rating value={project.stats.rating} readOnly size="small" />
                          <Typography variant="caption" sx={{ ml: 1 }}>
                            {project.stats.rating.toFixed(1)}
                          </Typography>
                        </Box>
                        <Typography variant="caption" color="text.secondary">
                          {project.stats.downloads} downloads
                        </Typography>
                      </Box>
                    </CardContent>
                    <CardActions>
                      <Button size="small" onClick={() => handleViewDetails(project)}>
                        View Details
                      </Button>
                      {project.demo_url && (
                        <Button
                          size="small"
                          startIcon={<OpenIcon />}
                          href={project.demo_url}
                          target="_blank"
                        >
                          Demo
                        </Button>
                      )}
                      {project.source_url && (
                        <Button
                          size="small"
                          startIcon={<CodeIcon />}
                          href={project.source_url}
                          target="_blank"
                        >
                          Source
                        </Button>
                      )}
                    </CardActions>
                  </Card>
                </Grid>
              ))}
            </Grid>
          )}
        </>
      )}

      {activeTab === 1 && (
        <Grid container spacing={3}>
          {stories.map((story) => (
            <Grid item xs={12} md={6} key={story.id}>
              <Card>
                <CardContent>
                  <Typography variant="h5" gutterBottom>
                    {story.title}
                  </Typography>
                  <Typography variant="subtitle1" color="text.secondary" gutterBottom>
                    {story.company} • {story.industry}
                  </Typography>
                  <Divider sx={{ my: 2 }} />
                  <Typography variant="subtitle2" gutterBottom>
                    Challenge
                  </Typography>
                  <Typography variant="body2" paragraph>
                    {story.challenge}
                  </Typography>
                  <Typography variant="subtitle2" gutterBottom>
                    Solution
                  </Typography>
                  <Typography variant="body2" paragraph>
                    {story.solution}
                  </Typography>
                  <Typography variant="subtitle2" gutterBottom>
                    Results
                  </Typography>
                  <ul>
                    {story.results.map((result, idx) => (
                      <li key={idx}>
                        <Typography variant="body2">{result}</Typography>
                      </li>
                    ))}
                  </ul>
                  <Typography variant="caption" color="text.secondary" sx={{ mt: 2, display: 'block' }}>
                    {story.author}, {story.role} • {new Date(story.date).toLocaleDateString()}
                  </Typography>
                </CardContent>
              </Card>
            </Grid>
          ))}
        </Grid>
      )}

      {/* Project Details Dialog */}
      <Dialog open={detailsOpen} onClose={() => setDetailsOpen(false)} maxWidth="md" fullWidth>
        {selectedProject && (
          <>
            <DialogTitle>{selectedProject.title}</DialogTitle>
            <DialogContent>
              <Box sx={{ mb: 2 }}>
                <Box sx={{ display: 'flex', alignItems: 'center', gap: 2, mb: 2 }}>
                  <Avatar>{selectedProject.author.charAt(0)}</Avatar>
                  <Box>
                    <Typography variant="subtitle1">{selectedProject.author}</Typography>
                    <Typography variant="caption" color="text.secondary">
                      {selectedProject.category}
                    </Typography>
                  </Box>
                </Box>
                {selectedProject.screenshot && (
                  <Box
                    component="img"
                    src={selectedProject.screenshot}
                    alt={selectedProject.title}
                    sx={{ width: '100%', borderRadius: 1, mb: 2 }}
                  />
                )}
                <Typography variant="body1" paragraph>
                  {selectedProject.description}
                </Typography>
                <Box sx={{ display: 'flex', gap: 1, flexWrap: 'wrap', mb: 2 }}>
                  {selectedProject.tags.map((tag) => (
                    <Chip key={tag} label={tag} size="small" />
                  ))}
                </Box>
                <Box sx={{ display: 'flex', gap: 3, mb: 2 }}>
                  <Box>
                    <Typography variant="caption" color="text.secondary">
                      Downloads
                    </Typography>
                    <Typography variant="h6">{selectedProject.stats.downloads}</Typography>
                  </Box>
                  <Box>
                    <Typography variant="caption" color="text.secondary">
                      Stars
                    </Typography>
                    <Typography variant="h6">{selectedProject.stats.stars}</Typography>
                  </Box>
                  <Box>
                    <Typography variant="caption" color="text.secondary">
                      Rating
                    </Typography>
                    <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                      <Rating value={selectedProject.stats.rating} readOnly size="small" />
                      <Typography variant="body2">{selectedProject.stats.rating.toFixed(1)}</Typography>
                    </Box>
                  </Box>
                </Box>
                {selectedProject.testimonials.length > 0 && (
                  <Box>
                    <Typography variant="subtitle2" gutterBottom>
                      Testimonials
                    </Typography>
                    {selectedProject.testimonials.map((testimonial, idx) => (
                      <Card key={idx} variant="outlined" sx={{ p: 2, mb: 1 }}>
                        <Typography variant="body2" paragraph>
                          "{testimonial.text}"
                        </Typography>
                        <Typography variant="caption" color="text.secondary">
                          — {testimonial.author}
                          {testimonial.company && `, ${testimonial.company}`}
                        </Typography>
                      </Card>
                    ))}
                  </Box>
                )}
              </Box>
            </DialogContent>
            <DialogActions>
              {selectedProject.demo_url && (
                <Button href={selectedProject.demo_url} target="_blank" startIcon={<OpenIcon />}>
                  View Demo
                </Button>
              )}
              {selectedProject.source_url && (
                <Button href={selectedProject.source_url} target="_blank" startIcon={<CodeIcon />}>
                  View Source
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
