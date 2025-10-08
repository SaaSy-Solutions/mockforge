/**
 * Version Control Panel
 *
 * Provides Git-like version control UI for orchestrations with
 * commit history, branching, diffing, and merging capabilities.
 */

import React, { useState, useEffect, useCallback } from 'react';
import {
  Box,
  Card,
  CardContent,
  Typography,
  Button,
  List,
  ListItem,
  ListItemText,
  ListItemIcon,
  Chip,
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  TextField,
  Grid,
  IconButton,
  Tooltip,
  Tabs,
  Tab,
  Divider,
  Alert,
} from '@mui/material';
import {
  AccountTree as BranchIcon,
  Commit as CommitIcon,
  CompareArrows as DiffIcon,
  Add as AddIcon,
  History as HistoryIcon,
  FileDownload as DownloadIcon,
  FileUpload as UploadIcon,
  PlayArrow as CheckoutIcon,
  Lock as ProtectedIcon,
} from '@mui/icons-material';

interface Commit {
  id: string;
  parentId?: string;
  author: string;
  email: string;
  message: string;
  timestamp: Date;
  contentHash: string;
}

interface Branch {
  name: string;
  headCommitId: string;
  createdAt: Date;
  createdBy: string;
  protected: boolean;
}

interface DiffChange {
  path: string;
  changeType: 'added' | 'modified' | 'deleted';
  oldValue?: any;
  newValue?: any;
}

interface Diff {
  fromCommit: string;
  toCommit: string;
  changes: DiffChange[];
  stats: {
    additions: number;
    deletions: number;
    modifications: number;
  };
}

export const VersionControlPanel: React.FC<{ orchestrationId: string }> = ({
  orchestrationId,
}) => {
  const [currentTab, setCurrentTab] = useState(0);
  const [commits, setCommits] = useState<Commit[]>([]);
  const [branches, setBranches] = useState<Branch[]>([]);
  const [currentBranch, setCurrentBranch] = useState<string>('main');
  const [selectedCommits, setSelectedCommits] = useState<string[]>([]);
  const [diff, setDiff] = useState<Diff | null>(null);

  // Dialogs
  const [commitDialogOpen, setCommitDialogOpen] = useState(false);
  const [branchDialogOpen, setBranchDialogOpen] = useState(false);
  const [diffDialogOpen, setDiffDialogOpen] = useState(false);

  // Form states
  const [commitMessage, setCommitMessage] = useState('');
  const [newBranchName, setNewBranchName] = useState('');

  // Load data
  useEffect(() => {
    loadHistory();
    loadBranches();
  }, [orchestrationId]);

  const loadHistory = async () => {
    try {
      const response = await fetch(`/api/chaos/orchestration/${orchestrationId}/history`);
      if (response.ok) {
        const data = await response.json();
        setCommits(data.commits || []);
        setCurrentBranch(data.currentBranch || 'main');
      }
    } catch (error) {
      console.error('Failed to load history:', error);
    }
  };

  const loadBranches = async () => {
    try {
      const response = await fetch(`/api/chaos/orchestration/${orchestrationId}/branches`);
      if (response.ok) {
        const data = await response.json();
        setBranches(data.branches || []);
      }
    } catch (error) {
      console.error('Failed to load branches:', error);
    }
  };

  const handleCommit = async () => {
    try {
      const response = await fetch(`/api/chaos/orchestration/${orchestrationId}/commit`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          message: commitMessage,
        }),
      });

      if (response.ok) {
        setCommitDialogOpen(false);
        setCommitMessage('');
        loadHistory();
      }
    } catch (error) {
      console.error('Failed to create commit:', error);
    }
  };

  const handleCreateBranch = async () => {
    try {
      const response = await fetch(`/api/chaos/orchestration/${orchestrationId}/branches`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          name: newBranchName,
          fromCommit: selectedCommits[0],
        }),
      });

      if (response.ok) {
        setBranchDialogOpen(false);
        setNewBranchName('');
        loadBranches();
      }
    } catch (error) {
      console.error('Failed to create branch:', error);
    }
  };

  const handleCheckout = async (branchName: string) => {
    try {
      const response = await fetch(`/api/chaos/orchestration/${orchestrationId}/checkout`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ branch: branchName }),
      });

      if (response.ok) {
        setCurrentBranch(branchName);
        loadHistory();
      }
    } catch (error) {
      console.error('Failed to checkout branch:', error);
    }
  };

  const handleDiff = async () => {
    if (selectedCommits.length !== 2) return;

    try {
      const response = await fetch(
        `/api/chaos/orchestration/${orchestrationId}/diff?from=${selectedCommits[0]}&to=${selectedCommits[1]}`
      );

      if (response.ok) {
        const diffData = await response.json();
        setDiff(diffData);
        setDiffDialogOpen(true);
      }
    } catch (error) {
      console.error('Failed to get diff:', error);
    }
  };

  const toggleCommitSelection = (commitId: string) => {
    setSelectedCommits((prev) => {
      if (prev.includes(commitId)) {
        return prev.filter((id) => id !== commitId);
      } else if (prev.length < 2) {
        return [...prev, commitId];
      }
      return [prev[1], commitId];
    });
  };

  const getChangeColor = (changeType: string) => {
    switch (changeType) {
      case 'added':
        return 'success';
      case 'deleted':
        return 'error';
      case 'modified':
        return 'warning';
      default:
        return 'default';
    }
  };

  return (
    <Box>
      {/* Header */}
      <Box sx={{ mb: 3, display: 'flex', alignItems: 'center', gap: 2 }}>
        <Chip
          icon={<BranchIcon />}
          label={`Current: ${currentBranch}`}
          color="primary"
          variant="outlined"
        />
        <Box sx={{ flexGrow: 1 }} />
        <Button
          startIcon={<AddIcon />}
          variant="outlined"
          onClick={() => setCommitDialogOpen(true)}
        >
          Commit
        </Button>
        <Button
          startIcon={<BranchIcon />}
          variant="outlined"
          onClick={() => setBranchDialogOpen(true)}
          disabled={selectedCommits.length === 0}
        >
          New Branch
        </Button>
        {selectedCommits.length === 2 && (
          <Button
            startIcon={<DiffIcon />}
            variant="contained"
            onClick={handleDiff}
          >
            Compare
          </Button>
        )}
      </Box>

      {selectedCommits.length > 0 && (
        <Alert severity="info" sx={{ mb: 2 }}>
          {selectedCommits.length} commit{selectedCommits.length > 1 ? 's' : ''} selected
          {selectedCommits.length === 2 && ' - Click "Compare" to view differences'}
        </Alert>
      )}

      {/* Tabs */}
      <Card>
        <Tabs value={currentTab} onChange={(_, v) => setCurrentTab(v)}>
          <Tab icon={<HistoryIcon />} label="History" />
          <Tab icon={<BranchIcon />} label="Branches" />
        </Tabs>

        <CardContent>
          {/* History Tab */}
          {currentTab === 0 && (
            <List>
              {commits.length === 0 && (
                <Typography variant="body2" color="text.secondary" sx={{ p: 2 }}>
                  No commits yet
                </Typography>
              )}
              {commits.map((commit, index) => (
                <React.Fragment key={commit.id}>
                  <ListItem
                    button
                    selected={selectedCommits.includes(commit.id)}
                    onClick={() => toggleCommitSelection(commit.id)}
                  >
                    <ListItemIcon>
                      <CommitIcon
                        color={selectedCommits.includes(commit.id) ? 'primary' : 'inherit'}
                      />
                    </ListItemIcon>
                    <ListItemText
                      primary={
                        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                          <Typography variant="subtitle1">{commit.message}</Typography>
                          <Chip label={commit.id.substring(0, 7)} size="small" />
                        </Box>
                      }
                      secondary={
                        <Box>
                          <Typography variant="caption" display="block">
                            {commit.author} ({commit.email})
                          </Typography>
                          <Typography variant="caption" color="text.secondary">
                            {new Date(commit.timestamp).toLocaleString()}
                          </Typography>
                        </Box>
                      }
                    />
                  </ListItem>
                  {index < commits.length - 1 && <Divider />}
                </React.Fragment>
              ))}
            </List>
          )}

          {/* Branches Tab */}
          {currentTab === 1 && (
            <List>
              {branches.map((branch) => (
                <ListItem
                  key={branch.name}
                  secondaryAction={
                    currentBranch !== branch.name && (
                      <Tooltip title="Checkout">
                        <IconButton
                          edge="end"
                          onClick={() => handleCheckout(branch.name)}
                          disabled={branch.protected && currentBranch === 'main'}
                        >
                          <CheckoutIcon />
                        </IconButton>
                      </Tooltip>
                    )
                  }
                >
                  <ListItemIcon>
                    {branch.protected ? (
                      <ProtectedIcon color="warning" />
                    ) : (
                      <BranchIcon />
                    )}
                  </ListItemIcon>
                  <ListItemText
                    primary={
                      <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                        <Typography variant="subtitle1">{branch.name}</Typography>
                        {currentBranch === branch.name && (
                          <Chip label="Current" color="primary" size="small" />
                        )}
                        {branch.protected && (
                          <Chip label="Protected" color="warning" size="small" />
                        )}
                      </Box>
                    }
                    secondary={
                      <Box>
                        <Typography variant="caption" display="block">
                          Created by {branch.createdBy}
                        </Typography>
                        <Typography variant="caption" color="text.secondary">
                          {new Date(branch.createdAt).toLocaleString()}
                        </Typography>
                      </Box>
                    }
                  />
                </ListItem>
              ))}
            </List>
          )}
        </CardContent>
      </Card>

      {/* Commit Dialog */}
      <Dialog open={commitDialogOpen} onClose={() => setCommitDialogOpen(false)} maxWidth="sm" fullWidth>
        <DialogTitle>Create Commit</DialogTitle>
        <DialogContent>
          <TextField
            autoFocus
            margin="dense"
            label="Commit Message"
            fullWidth
            multiline
            rows={3}
            value={commitMessage}
            onChange={(e) => setCommitMessage(e.target.value)}
          />
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setCommitDialogOpen(false)}>Cancel</Button>
          <Button onClick={handleCommit} variant="contained" disabled={!commitMessage}>
            Commit
          </Button>
        </DialogActions>
      </Dialog>

      {/* Branch Dialog */}
      <Dialog open={branchDialogOpen} onClose={() => setBranchDialogOpen(false)} maxWidth="sm" fullWidth>
        <DialogTitle>Create Branch</DialogTitle>
        <DialogContent>
          <TextField
            autoFocus
            margin="dense"
            label="Branch Name"
            fullWidth
            value={newBranchName}
            onChange={(e) => setNewBranchName(e.target.value)}
            helperText={
              selectedCommits.length > 0
                ? `Branch from commit: ${selectedCommits[0].substring(0, 7)}`
                : 'Branch from current HEAD'
            }
          />
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setBranchDialogOpen(false)}>Cancel</Button>
          <Button onClick={handleCreateBranch} variant="contained" disabled={!newBranchName}>
            Create
          </Button>
        </DialogActions>
      </Dialog>

      {/* Diff Dialog */}
      <Dialog open={diffDialogOpen} onClose={() => setDiffDialogOpen(false)} maxWidth="md" fullWidth>
        <DialogTitle>
          Diff: {diff?.fromCommit.substring(0, 7)} → {diff?.toCommit.substring(0, 7)}
        </DialogTitle>
        <DialogContent>
          {diff && (
            <>
              <Grid container spacing={2} sx={{ mb: 2 }}>
                <Grid item xs={4}>
                  <Chip
                    label={`+${diff.stats.additions} additions`}
                    color="success"
                    size="small"
                  />
                </Grid>
                <Grid item xs={4}>
                  <Chip
                    label={`~${diff.stats.modifications} modifications`}
                    color="warning"
                    size="small"
                  />
                </Grid>
                <Grid item xs={4}>
                  <Chip
                    label={`-${diff.stats.deletions} deletions`}
                    color="error"
                    size="small"
                  />
                </Grid>
              </Grid>

              <List>
                {diff.changes.map((change, index) => (
                  <React.Fragment key={index}>
                    <ListItem>
                      <ListItemText
                        primary={
                          <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                            <Chip
                              label={change.changeType}
                              color={getChangeColor(change.changeType) as any}
                              size="small"
                            />
                            <Typography variant="body2" fontFamily="monospace">
                              {change.path}
                            </Typography>
                          </Box>
                        }
                        secondary={
                          <Box sx={{ mt: 1 }}>
                            {change.oldValue && (
                              <Typography
                                variant="caption"
                                component="div"
                                sx={{ color: 'error.main' }}
                              >
                                - {JSON.stringify(change.oldValue)}
                              </Typography>
                            )}
                            {change.newValue && (
                              <Typography
                                variant="caption"
                                component="div"
                                sx={{ color: 'success.main' }}
                              >
                                + {JSON.stringify(change.newValue)}
                              </Typography>
                            )}
                          </Box>
                        }
                      />
                    </ListItem>
                    {index < diff.changes.length - 1 && <Divider />}
                  </React.Fragment>
                ))}
              </List>
            </>
          )}
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setDiffDialogOpen(false)}>Close</Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
};

export default VersionControlPanel;
