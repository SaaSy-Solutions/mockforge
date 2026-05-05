/**
 * CloudPluginsBetaCta — demand-validation CTA for cloud-runtime plugins.
 *
 * Phase 0 of the cloud-plugins initiative: before committing to the
 * 4–6 weeks of runtime + metering work, ship a "Request beta access"
 * banner on the cloud /plugin-registry page and gate the engineering
 * spend on signups. A user submits once (UPSERT on user_id) with an
 * optional free-text use case. After submitting, the banner switches
 * to a thank-you state so the page doesn't keep prompting.
 *
 * Hides itself entirely in self-hosted (local) mode — there is no
 * cloud runtime to attach plugins to in that build.
 */
import React, { useState } from 'react';
import {
  Alert,
  Box,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  IconButton,
  Snackbar,
  Stack,
  TextField,
  Typography,
} from '@mui/material';
import {
  Cloud as CloudIcon,
  Close as CloseIcon,
  CheckCircle as CheckCircleIcon,
} from '@mui/icons-material';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { isCloudMode } from '../../utils/cloudMode';
import {
  cloudPluginsApi,
  type BetaInterestStatus,
} from '../../services/api/cloudPlugins';

const QUERY_KEY = ['cloud-plugins', 'beta-interest', 'me'];
const USE_CASE_MAX = 2_000;

export const CloudPluginsBetaCta: React.FC = () => {
  if (!isCloudMode()) return null;
  return <Inner />;
};

const Inner: React.FC = () => {
  const queryClient = useQueryClient();
  const statusQuery = useQuery<BetaInterestStatus>({
    queryKey: QUERY_KEY,
    queryFn: () => cloudPluginsApi.getMyBetaInterest(),
    staleTime: 5 * 60_000,
  });

  const [dialogOpen, setDialogOpen] = useState(false);
  const [useCase, setUseCase] = useState('');
  const [snackbarMsg, setSnackbarMsg] = useState<string | null>(null);

  const submit = useMutation({
    mutationFn: (text: string) =>
      cloudPluginsApi.submitBetaInterest({
        use_case: text.trim() ? text.trim() : undefined,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: QUERY_KEY });
      setDialogOpen(false);
      setSnackbarMsg("You're on the beta list. We'll be in touch.");
    },
  });

  const openDialog = () => {
    setUseCase(statusQuery.data?.use_case ?? '');
    setDialogOpen(true);
  };

  // Don't render until we know whether the user has signed up — avoids
  // a flash of the CTA followed by the thank-you state on every page
  // load. Errors are soft-failed (banner just doesn't show).
  if (statusQuery.isLoading || statusQuery.isError || !statusQuery.data) {
    return null;
  }

  const status = statusQuery.data;

  return (
    <>
      {status.signed_up ? (
        <SignedUpBanner onUpdate={openDialog} />
      ) : (
        <CtaBanner onClick={openDialog} />
      )}

      <Dialog
        open={dialogOpen}
        onClose={() => setDialogOpen(false)}
        maxWidth="sm"
        fullWidth
        aria-labelledby="cloud-plugins-beta-dialog-title"
      >
        <DialogTitle id="cloud-plugins-beta-dialog-title">
          Cloud Plugins — beta access
        </DialogTitle>
        <DialogContent>
          <Stack spacing={2} sx={{ mt: 1 }}>
            <Typography variant="body2" color="text.secondary">
              We're exploring a per-tenant plugin runtime that would let you
              attach signed WASM extensions to your hosted mocks — request
              transformers, custom auth, response shaping, etc. — without
              self-hosting MockForge.
            </Typography>
            <Typography variant="body2" color="text.secondary">
              If that sounds useful, tell us roughly what you'd build. It
              shapes which capabilities ship first and who we invite to the
              private beta.
            </Typography>
            <TextField
              label="What would you build with cloud plugins? (optional)"
              multiline
              minRows={3}
              maxRows={8}
              fullWidth
              value={useCase}
              onChange={(e) => setUseCase(e.target.value.slice(0, USE_CASE_MAX))}
              helperText={`${useCase.length} / ${USE_CASE_MAX}`}
              inputProps={{ 'data-testid': 'beta-use-case-input' }}
            />
            {submit.isError && (
              <Alert severity="error">
                Something went wrong. Try again in a moment.
              </Alert>
            )}
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setDialogOpen(false)} disabled={submit.isPending}>
            Cancel
          </Button>
          <Button
            variant="contained"
            disabled={submit.isPending}
            onClick={() => submit.mutate(useCase)}
            data-testid="beta-submit-button"
          >
            {status.signed_up ? 'Update' : 'Request beta access'}
          </Button>
        </DialogActions>
      </Dialog>

      <Snackbar
        open={snackbarMsg !== null}
        autoHideDuration={4000}
        onClose={() => setSnackbarMsg(null)}
        message={snackbarMsg}
      />
    </>
  );
};

const CtaBanner: React.FC<{ onClick: () => void }> = ({ onClick }) => (
  <Box
    sx={{
      display: 'flex',
      alignItems: 'center',
      gap: 2,
      p: 2,
      mb: 3,
      borderRadius: 2,
      background: 'linear-gradient(90deg, rgba(59,130,246,0.08), rgba(168,85,247,0.08))',
      border: '1px solid',
      borderColor: 'divider',
    }}
    data-testid="cloud-plugins-beta-cta"
  >
    <CloudIcon sx={{ color: 'primary.main' }} />
    <Box sx={{ flex: 1 }}>
      <Typography variant="subtitle1" sx={{ fontWeight: 600 }}>
        Coming soon: run plugins in your cloud workspace
      </Typography>
      <Typography variant="body2" color="text.secondary">
        Help us decide what ships first — request beta access in 30 seconds.
      </Typography>
    </Box>
    <Button variant="contained" size="small" onClick={onClick}>
      Request beta access
    </Button>
  </Box>
);

const SignedUpBanner: React.FC<{ onUpdate: () => void }> = ({ onUpdate }) => {
  const [dismissed, setDismissed] = useState(false);
  if (dismissed) return null;

  return (
    <Box
      sx={{
        display: 'flex',
        alignItems: 'center',
        gap: 2,
        p: 1.5,
        mb: 3,
        borderRadius: 2,
        background: (theme) =>
          theme.palette.mode === 'dark'
            ? 'rgba(34,197,94,0.08)'
            : 'rgba(34,197,94,0.06)',
        border: '1px solid',
        borderColor: 'success.light',
      }}
      data-testid="cloud-plugins-beta-signed-up"
    >
      <CheckCircleIcon sx={{ color: 'success.main' }} />
      <Box sx={{ flex: 1 }}>
        <Typography variant="body2">
          You're on the cloud-plugins beta list. We'll reach out as the runtime
          ships.
        </Typography>
      </Box>
      <Button size="small" onClick={onUpdate}>
        Update
      </Button>
      <IconButton
        size="small"
        onClick={() => setDismissed(true)}
        aria-label="Dismiss"
      >
        <CloseIcon fontSize="small" />
      </IconButton>
    </Box>
  );
};
