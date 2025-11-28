/**
 * Publish Scenario Modal
 *
 * Form for publishing scenarios to the marketplace
 */

import React, { useState } from 'react';
import {
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  TextField,
  Button,
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

interface PublishScenarioModalProps {
  open: boolean;
  onClose: () => void;
  onSuccess?: () => void;
}

export const PublishScenarioModal: React.FC<PublishScenarioModalProps> = ({
  open,
  onClose,
  onSuccess,
}) => {
  const [manifest, setManifest] = useState(`{
  "name": "",
  "version": "1.0.0",
  "description": "",
  "category": "other",
  "license": "MIT",
  "compatibility": {
    "min_version": "0.1.0"
  }
}`);
  const [packageFile, setPackageFile] = useState<File | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

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
      // Validate manifest JSON
      let manifestJson;
      try {
        manifestJson = JSON.parse(manifest);
      } catch {
        throw new Error('Invalid JSON in manifest');
      }

      if (!manifestJson.name || !manifestJson.version) {
        throw new Error('Manifest must include name and version');
      }

      if (!packageFile) {
        throw new Error('Please select a package file');
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
        manifest: JSON.stringify(manifestJson),
        package: base64Package,
        checksum,
        size: fileBuffer.byteLength,
      };

      // Get auth token
      const token = localStorage.getItem('auth_token');
      if (!token) {
        throw new Error('Not authenticated. Please log in.');
      }

      // Submit to API
      const response = await fetch('/api/v1/scenarios/publish', {
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
      setManifest(`{
  "name": "",
  "version": "1.0.0",
  "description": "",
  "category": "other",
  "license": "MIT",
  "compatibility": {
    "min_version": "0.1.0"
  }
}`);
      setPackageFile(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to publish scenario');
    } finally {
      setLoading(false);
    }
  };

  return (
    <Dialog open={open} onClose={onClose} maxWidth="md" fullWidth>
      <DialogTitle>
        <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <Typography variant="h6">Publish Scenario</Typography>
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
            Scenarios are published using a manifest file that describes the scenario metadata.
          </Alert>

          <TextField
            label="Manifest (JSON)"
            required
            fullWidth
            multiline
            rows={12}
            value={manifest}
            onChange={(e) => setManifest(e.target.value)}
            placeholder='{"name": "my-scenario", "version": "1.0.0", ...}'
            helperText="Scenario manifest in JSON format. Must include: name, version, description, category, license"
          />

          <Box>
            <Typography variant="body2" sx={{ mb: 1 }}>
              Scenario Package (tar.gz)
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

          <Alert severity="warning">
            <Typography variant="body2">
              <strong>Manifest Requirements:</strong>
            </Typography>
            <Typography variant="body2" component="ul" sx={{ mt: 1, pl: 2 }}>
              <li>name: Scenario name (required)</li>
              <li>version: Semantic version (required)</li>
              <li>description: Scenario description (required)</li>
              <li>category: One of: network-chaos, service-failure, load-testing, etc.</li>
              <li>license: License identifier (e.g., MIT, Apache-2.0)</li>
              <li>compatibility.min_version: Minimum MockForge version</li>
            </Typography>
          </Alert>
        </Box>
      </DialogContent>

      <DialogActions>
        <Button onClick={onClose} disabled={loading}>
          Cancel
        </Button>
        <Button
          variant="contained"
          onClick={handleSubmit}
          disabled={loading || !manifest || !packageFile}
          startIcon={<UploadIcon />}
        >
          {loading ? 'Publishing...' : 'Publish Scenario'}
        </Button>
      </DialogActions>
    </Dialog>
  );
};
