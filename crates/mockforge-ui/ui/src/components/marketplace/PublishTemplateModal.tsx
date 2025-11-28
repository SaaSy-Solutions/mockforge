/**
 * Publish Template Modal
 *
 * Form for publishing templates to the marketplace
 */

import React, { useState } from 'react';
import {
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  TextField,
  Button,
  FormControl,
  InputLabel,
  Select,
  MenuItem,
  Alert,
  Box,
  Typography,
  LinearProgress,
  IconButton,
} from '@mui/material';
import {
  Upload as UploadIcon,
  Close as CloseIcon,
} from '@mui/icons-material';

interface PublishTemplateModalProps {
  open: boolean;
  onClose: () => void;
  onSuccess?: () => void;
}

const TEMPLATE_CATEGORIES = [
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

export const PublishTemplateModal: React.FC<PublishTemplateModalProps> = ({
  open,
  onClose,
  onSuccess,
}) => {
  const [formData, setFormData] = useState({
    name: '',
    slug: '',
    description: '',
    version: '1.0.0',
    category: 'other',
    content_json: '{}',
  });
  const [packageFile, setPackageFile] = useState<File | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleInputChange = (field: string, value: string) => {
    setFormData((prev) => ({ ...prev, [field]: value }));

    // Auto-generate slug from name
    if (field === 'name') {
      const slug = value
        .toLowerCase()
        .replace(/[^a-z0-9]+/g, '-')
        .replace(/^-+|-+$/g, '');
      setFormData((prev) => ({ ...prev, slug }));
    }
  };

  const handleFileSelect = (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (file) {
      setPackageFile(file);
    }
  };

  const calculateChecksum = async (data: ArrayBuffer): Promise<string> => {
    const hashBuffer = await crypto.subtle.digest('SHA-256', data);
    const hashArray = Array.from(new Uint8Array(hashBuffer));
    return hashArray.map((b) => b.toString(16).padStart(2, '0')).join('');
  };

  const handleSubmit = async () => {
    setError(null);
    setLoading(true);

    try {
      // Validate form
      if (!formData.name || !formData.description || !formData.version) {
        throw new Error('Please fill in all required fields');
      }

      if (!packageFile) {
        throw new Error('Please select a package file');
      }

      // Validate JSON
      let contentJson;
      try {
        contentJson = JSON.parse(formData.content_json);
      } catch {
        throw new Error('Invalid JSON in content field');
      }

      // Read and encode package file
      const fileBuffer = await packageFile.arrayBuffer();
      const base64Package = btoa(
        String.fromCharCode(...new Uint8Array(fileBuffer))
      );

      // Calculate checksum
      const checksum = await calculateChecksum(fileBuffer);

      // Prepare request
      const request = {
        name: formData.name,
        slug: formData.slug || formData.name.toLowerCase().replace(/[^a-z0-9]+/g, '-'),
        description: formData.description,
        version: formData.version,
        category: formData.category,
        content_json: contentJson,
        package: base64Package,
        checksum,
        file_size: fileBuffer.byteLength,
      };

      // Get auth token
      const token = localStorage.getItem('auth_token');
      if (!token) {
        throw new Error('Not authenticated. Please log in.');
      }

      // Submit to API
      const response = await fetch('/api/v1/templates/publish', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify(request),
      });

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({ error: 'Unknown error' }));
        throw new Error(errorData.error || `Failed to publish: ${response.statusText}`);
      }

      const result = await response.json();

      // Success
      if (onSuccess) {
        onSuccess();
      }
      onClose();

      // Reset form
      setFormData({
        name: '',
        slug: '',
        description: '',
        version: '1.0.0',
        category: 'other',
        content_json: '{}',
      });
      setPackageFile(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to publish template');
    } finally {
      setLoading(false);
    }
  };

  return (
    <Dialog open={open} onClose={onClose} maxWidth="md" fullWidth>
      <DialogTitle>
        <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <Typography variant="h6">Publish Template</Typography>
          <IconButton onClick={onClose} size="small">
            <CloseIcon />
          </IconButton>
        </Box>
      </DialogTitle>

      <DialogContent>
        {error && (
          <Alert severity="error" sx={{ mb: 2 }}>
            {error}
          </Alert>
        )}

        {loading && <LinearProgress sx={{ mb: 2 }} />}

        <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2, mt: 1 }}>
          <TextField
            label="Template Name"
            required
            fullWidth
            value={formData.name}
            onChange={(e) => handleInputChange('name', e.target.value)}
            placeholder="e.g., Network Latency Chaos"
          />

          <TextField
            label="Slug"
            fullWidth
            value={formData.slug}
            onChange={(e) => handleInputChange('slug', e.target.value)}
            placeholder="auto-generated-from-name"
            helperText="URL-friendly identifier (auto-generated from name)"
          />

          <TextField
            label="Description"
            required
            fullWidth
            multiline
            rows={3}
            value={formData.description}
            onChange={(e) => handleInputChange('description', e.target.value)}
            placeholder="Describe what this template does..."
          />

          <Box sx={{ display: 'flex', gap: 2 }}>
            <TextField
              label="Version"
              required
              fullWidth
              value={formData.version}
              onChange={(e) => handleInputChange('version', e.target.value)}
              placeholder="1.0.0"
            />

            <FormControl fullWidth>
              <InputLabel>Category</InputLabel>
              <Select
                value={formData.category}
                label="Category"
                onChange={(e) => handleInputChange('category', e.target.value)}
              >
                {TEMPLATE_CATEGORIES.map((cat) => (
                  <MenuItem key={cat.value} value={cat.value}>
                    {cat.label}
                  </MenuItem>
                ))}
              </Select>
            </FormControl>
          </Box>

          <TextField
            label="Template Content (JSON)"
            required
            fullWidth
            multiline
            rows={6}
            value={formData.content_json}
            onChange={(e) => handleInputChange('content_json', e.target.value)}
            placeholder='{"steps": [], "config": {}}'
            helperText="Template configuration in JSON format"
          />

          <Box>
            <Typography variant="body2" sx={{ mb: 1 }}>
              Package File (tar.gz)
            </Typography>
            <Button
              variant="outlined"
              component="label"
              startIcon={<UploadIcon />}
              fullWidth
            >
              {packageFile ? packageFile.name : 'Select Package File'}
              <input
                type="file"
                hidden
                accept=".tar.gz,.tgz"
                onChange={handleFileSelect}
              />
            </Button>
            {packageFile && (
              <Typography variant="caption" color="text.secondary" sx={{ mt: 1, display: 'block' }}>
                Size: {(packageFile.size / 1024).toFixed(2)} KB
              </Typography>
            )}
          </Box>
        </Box>
      </DialogContent>

      <DialogActions>
        <Button onClick={onClose} disabled={loading}>
          Cancel
        </Button>
        <Button
          variant="contained"
          onClick={handleSubmit}
          disabled={loading || !formData.name || !formData.description || !packageFile}
          startIcon={<UploadIcon />}
        >
          {loading ? 'Publishing...' : 'Publish Template'}
        </Button>
      </DialogActions>
    </Dialog>
  );
};
