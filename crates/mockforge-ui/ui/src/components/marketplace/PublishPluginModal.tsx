/**
 * Publish Plugin Modal
 *
 * Form for publishing plugins to the marketplace
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
  Chip,
} from '@mui/material';
import {
  Upload as UploadIcon,
  Close as CloseIcon,
  Add as AddIcon,
} from '@mui/icons-material';

interface PublishPluginModalProps {
  open: boolean;
  onClose: () => void;
  onSuccess?: () => void;
}

const PLUGIN_CATEGORIES = [
  { value: 'auth', label: 'Authentication' },
  { value: 'template', label: 'Templates' },
  { value: 'response', label: 'Response' },
  { value: 'datasource', label: 'Data Source' },
  { value: 'middleware', label: 'Middleware' },
  { value: 'testing', label: 'Testing' },
  { value: 'observability', label: 'Observability' },
  { value: 'other', label: 'Other' },
];

const PLUGIN_LICENSES = [
  'MIT',
  'Apache-2.0',
  'BSD-2-Clause',
  'BSD-3-Clause',
  'GPL-2.0',
  'GPL-3.0',
  'LGPL-2.1',
  'LGPL-3.0',
  'MPL-2.0',
  'ISC',
  'Unlicense',
  'Other',
];

export const PublishPluginModal: React.FC<PublishPluginModalProps> = ({
  open,
  onClose,
  onSuccess,
}) => {
  const [formData, setFormData] = useState({
    name: '',
    version: '1.0.0',
    description: '',
    category: 'other',
    license: 'MIT',
    repository: '',
    homepage: '',
    tags: [] as string[],
    min_mockforge_version: '0.1.0',
  });
  const [tagInput, setTagInput] = useState('');
  const [wasmFile, setWasmFile] = useState<File | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleInputChange = (field: string, value: string) => {
    setFormData((prev) => ({ ...prev, [field]: value }));
  };

  const handleFileSelect = (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (file) {
      if (!file.name.endsWith('.wasm')) {
        setError('Please select a .wasm file');
        return;
      }
      setWasmFile(file);
      setError(null);
    }
  };

  const handleAddTag = () => {
    if (tagInput.trim() && !formData.tags.includes(tagInput.trim())) {
      setFormData((prev) => ({
        ...prev,
        tags: [...prev.tags, tagInput.trim()],
      }));
      setTagInput('');
    }
  };

  const handleRemoveTag = (tag: string) => {
    setFormData((prev) => ({
      ...prev,
      tags: prev.tags.filter((t) => t !== tag),
    }));
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

      if (!wasmFile) {
        throw new Error('Please select a WASM file');
      }

      // Read and encode WASM file
      const fileBuffer = await wasmFile.arrayBuffer();
      const base64Wasm = btoa(String.fromCharCode(...new Uint8Array(fileBuffer)));

      // Calculate checksum
      const checksum = await calculateChecksum(fileBuffer);

      // Prepare request
      const request = {
        name: formData.name,
        version: formData.version,
        description: formData.description,
        category: formData.category,
        license: formData.license,
        repository: formData.repository || null,
        homepage: formData.homepage || null,
        tags: formData.tags,
        checksum,
        file_size: fileBuffer.byteLength,
        wasm_data: base64Wasm,
        min_mockforge_version: formData.min_mockforge_version || null,
      };

      // Get auth token
      const token = localStorage.getItem('auth_token');
      if (!token) {
        throw new Error('Not authenticated. Please log in.');
      }

      // Submit to API
      const response = await fetch('/api/v1/plugins/publish', {
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
        version: '1.0.0',
        description: '',
        category: 'other',
        license: 'MIT',
        repository: '',
        homepage: '',
        tags: [],
        min_mockforge_version: '0.1.0',
      });
      setWasmFile(null);
      setTagInput('');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to publish plugin');
    } finally {
      setLoading(false);
    }
  };

  return (
    <Dialog open={open} onClose={onClose} maxWidth="md" fullWidth>
      <DialogTitle>
        <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <Typography variant="h6">Publish Plugin</Typography>
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
          <Alert severity="info" sx={{ mb: 1 }}>
            Plugins must be compiled to WebAssembly (WASM) format. Use the MockForge plugin SDK to build your plugin.
          </Alert>

          <TextField
            label="Plugin Name"
            required
            fullWidth
            value={formData.name}
            onChange={(e) => handleInputChange('name', e.target.value)}
            placeholder="e.g., jwt-auth-plugin"
            helperText="Unique identifier for your plugin"
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

            <FormControl fullWidth required>
              <InputLabel>Category</InputLabel>
              <Select
                value={formData.category}
                label="Category"
                onChange={(e) => handleInputChange('category', e.target.value)}
              >
                {PLUGIN_CATEGORIES.map((cat) => (
                  <MenuItem key={cat.value} value={cat.value}>
                    {cat.label}
                  </MenuItem>
                ))}
              </Select>
            </FormControl>
          </Box>

          <TextField
            label="Description"
            required
            fullWidth
            multiline
            rows={3}
            value={formData.description}
            onChange={(e) => handleInputChange('description', e.target.value)}
            placeholder="Describe what this plugin does..."
          />

          <Box sx={{ display: 'flex', gap: 2 }}>
            <FormControl fullWidth required>
              <InputLabel>License</InputLabel>
              <Select
                value={formData.license}
                label="License"
                onChange={(e) => handleInputChange('license', e.target.value)}
              >
                {PLUGIN_LICENSES.map((license) => (
                  <MenuItem key={license} value={license}>
                    {license}
                  </MenuItem>
                ))}
              </Select>
            </FormControl>

            <TextField
              label="Min MockForge Version"
              fullWidth
              value={formData.min_mockforge_version}
              onChange={(e) => handleInputChange('min_mockforge_version', e.target.value)}
              placeholder="0.1.0"
              helperText="Minimum required MockForge version"
            />
          </Box>

          <TextField
            label="Repository URL"
            fullWidth
            value={formData.repository}
            onChange={(e) => handleInputChange('repository', e.target.value)}
            placeholder="https://github.com/username/plugin-name"
            helperText="Git repository URL (optional)"
          />

          <TextField
            label="Homepage URL"
            fullWidth
            value={formData.homepage}
            onChange={(e) => handleInputChange('homepage', e.target.value)}
            placeholder="https://plugin-docs.example.com"
            helperText="Plugin documentation homepage (optional)"
          />

          <Box>
            <Typography variant="body2" sx={{ mb: 1 }}>
              Tags
            </Typography>
            <Box sx={{ display: 'flex', gap: 1, mb: 1, flexWrap: 'wrap' }}>
              {formData.tags.map((tag) => (
                <Chip
                  key={tag}
                  label={tag}
                  onDelete={() => handleRemoveTag(tag)}
                  size="small"
                />
              ))}
            </Box>
            <Box sx={{ display: 'flex', gap: 1 }}>
              <TextField
                size="small"
                placeholder="Add a tag"
                value={tagInput}
                onChange={(e) => setTagInput(e.target.value)}
                onKeyPress={(e) => {
                  if (e.key === 'Enter') {
                    e.preventDefault();
                    handleAddTag();
                  }
                }}
                sx={{ flexGrow: 1 }}
              />
              <Button
                size="small"
                variant="outlined"
                startIcon={<AddIcon />}
                onClick={handleAddTag}
              >
                Add
              </Button>
            </Box>
          </Box>

          <Box>
            <Typography variant="body2" sx={{ mb: 1 }}>
              WASM File (.wasm)
            </Typography>
            <Button
              variant="outlined"
              component="label"
              startIcon={<UploadIcon />}
              fullWidth
            >
              {wasmFile ? wasmFile.name : 'Select WASM File'}
              <input
                type="file"
                hidden
                accept=".wasm"
                onChange={handleFileSelect}
              />
            </Button>
            {wasmFile && (
              <Typography variant="caption" color="text.secondary" sx={{ mt: 1, display: 'block' }}>
                Size: {(wasmFile.size / 1024).toFixed(2)} KB
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
          disabled={loading || !formData.name || !formData.description || !wasmFile}
          startIcon={<UploadIcon />}
        >
          {loading ? 'Publishing...' : 'Publish Plugin'}
        </Button>
      </DialogActions>
    </Dialog>
  );
};
