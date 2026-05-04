import React, { useEffect, useState } from 'react';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import {
    Dialog,
    DialogContent,
    DialogHeader,
    DialogTitle,
    DialogDescription,
    DialogFooter,
    DialogClose,
} from '../ui/Dialog';
import { useAuthStore } from '../../stores/useAuthStore';
import { authApi, type TwoFactorSetup } from '../../services/authApi';
import { Shield, Bell, Lock, Mail, Copy, CheckCircle2, Download, Trash2 } from 'lucide-react';

interface AccountSettingsProps {
    open: boolean;
    onOpenChange: (open: boolean) => void;
}

type Banner =
    | { kind: 'success'; message: string }
    | { kind: 'error'; message: string }
    | null;

export function AccountSettings({ open, onOpenChange }: AccountSettingsProps) {
    const { user } = useAuthStore();

    // Password change state
    const [pwCurrent, setPwCurrent] = useState('');
    const [pwNew, setPwNew] = useState('');
    const [pwConfirm, setPwConfirm] = useState('');
    const [pwSubmitting, setPwSubmitting] = useState(false);
    const [pwErrors, setPwErrors] = useState<Record<string, string>>({});
    const [pwBanner, setPwBanner] = useState<Banner>(null);

    // 2FA state
    const [twoFactorEnabled, setTwoFactorEnabled] = useState(false);
    const [twoFactorLoading, setTwoFactorLoading] = useState(false);
    const [twoFactorSetup, setTwoFactorSetup] = useState<TwoFactorSetup | null>(null);
    const [twoFactorCode, setTwoFactorCode] = useState('');
    const [twoFactorBanner, setTwoFactorBanner] = useState<Banner>(null);
    const [showDisableForm, setShowDisableForm] = useState(false);
    const [disablePassword, setDisablePassword] = useState('');
    const [copiedCodes, setCopiedCodes] = useState(false);

    // Notification prefs state
    const [emailNotifications, setEmailNotifications] = useState(true);
    const [securityAlerts, setSecurityAlerts] = useState(true);
    const [notifSaving, setNotifSaving] = useState(false);
    const [notifBanner, setNotifBanner] = useState<Banner>(null);

    // GDPR state
    const [gdprBanner, setGdprBanner] = useState<Banner>(null);
    const [exporting, setExporting] = useState(false);
    const [showDeleteForm, setShowDeleteForm] = useState(false);
    const [deleteConfirm, setDeleteConfirm] = useState('');
    const [deleteReason, setDeleteReason] = useState('');
    const [deleting, setDeleting] = useState(false);

    // Hydrate state from server when the dialog opens
    useEffect(() => {
        if (!open || !user) return;
        let cancelled = false;
        setPwCurrent('');
        setPwNew('');
        setPwConfirm('');
        setPwErrors({});
        setPwBanner(null);
        setTwoFactorSetup(null);
        setTwoFactorCode('');
        setTwoFactorBanner(null);
        setShowDisableForm(false);
        setDisablePassword('');
        setCopiedCodes(false);
        setNotifBanner(null);
        setGdprBanner(null);
        setShowDeleteForm(false);
        setDeleteConfirm('');
        setDeleteReason('');

        authApi
            .getMe()
            .then((profile) => {
                if (cancelled) return;
                setTwoFactorEnabled(profile.two_factor_enabled);
                setEmailNotifications(profile.email_notifications);
                setSecurityAlerts(profile.security_alerts);
            })
            .catch((err) => {
                if (cancelled) return;
                setTwoFactorBanner({
                    kind: 'error',
                    message: err instanceof Error ? err.message : 'Failed to load account',
                });
            });

        return () => {
            cancelled = true;
        };
    }, [open, user]);

    const handlePasswordSubmit = async (e: React.FormEvent) => {
        e.preventDefault();
        const errors: Record<string, string> = {};
        if (!pwCurrent) errors.current = 'Current password is required';
        if (pwNew.length < 8) errors.new = 'Password must be at least 8 characters';
        if (pwNew !== pwConfirm) errors.confirm = 'Passwords do not match';
        if (pwNew && pwNew === pwCurrent) {
            errors.new = 'New password must differ from the current password';
        }
        setPwErrors(errors);
        if (Object.keys(errors).length > 0) return;

        setPwSubmitting(true);
        setPwBanner(null);
        try {
            const res = await authApi.changePassword(pwCurrent, pwNew);
            setPwBanner({ kind: 'success', message: res.message });
            setPwCurrent('');
            setPwNew('');
            setPwConfirm('');
        } catch (err) {
            setPwBanner({
                kind: 'error',
                message: err instanceof Error ? err.message : 'Failed to change password',
            });
        } finally {
            setPwSubmitting(false);
        }
    };

    const handleBeginSetup2FA = async () => {
        setTwoFactorLoading(true);
        setTwoFactorBanner(null);
        try {
            const setup = await authApi.setup2FA();
            setTwoFactorSetup(setup);
            setCopiedCodes(false);
        } catch (err) {
            setTwoFactorBanner({
                kind: 'error',
                message: err instanceof Error ? err.message : 'Failed to start 2FA setup',
            });
        } finally {
            setTwoFactorLoading(false);
        }
    };

    const handleCancelSetup2FA = () => {
        setTwoFactorSetup(null);
        setTwoFactorCode('');
        setTwoFactorBanner(null);
    };

    const handleVerify2FA = async (e: React.FormEvent) => {
        e.preventDefault();
        if (!twoFactorSetup) return;
        if (twoFactorCode.trim().length < 6) {
            setTwoFactorBanner({ kind: 'error', message: 'Enter the 6-digit code from your authenticator app' });
            return;
        }
        setTwoFactorLoading(true);
        setTwoFactorBanner(null);
        try {
            await authApi.verify2FASetup(
                twoFactorSetup.secret,
                twoFactorCode.trim(),
                twoFactorSetup.backup_codes,
            );
            setTwoFactorEnabled(true);
            setTwoFactorSetup(null);
            setTwoFactorCode('');
            setTwoFactorBanner({
                kind: 'success',
                message: 'Two-factor authentication is now enabled.',
            });
        } catch (err) {
            setTwoFactorBanner({
                kind: 'error',
                message: err instanceof Error ? err.message : 'Verification failed',
            });
        } finally {
            setTwoFactorLoading(false);
        }
    };

    const handleDisable2FA = async (e: React.FormEvent) => {
        e.preventDefault();
        if (!disablePassword) {
            setTwoFactorBanner({ kind: 'error', message: 'Enter your password to disable 2FA' });
            return;
        }
        setTwoFactorLoading(true);
        setTwoFactorBanner(null);
        try {
            await authApi.disable2FA(disablePassword);
            setTwoFactorEnabled(false);
            setShowDisableForm(false);
            setDisablePassword('');
            setTwoFactorBanner({
                kind: 'success',
                message: 'Two-factor authentication has been disabled.',
            });
        } catch (err) {
            setTwoFactorBanner({
                kind: 'error',
                message: err instanceof Error ? err.message : 'Failed to disable 2FA',
            });
        } finally {
            setTwoFactorLoading(false);
        }
    };

    const handleCopyBackupCodes = async () => {
        if (!twoFactorSetup) return;
        try {
            await navigator.clipboard.writeText(twoFactorSetup.backup_codes.join('\n'));
            setCopiedCodes(true);
            setTimeout(() => setCopiedCodes(false), 2000);
        } catch {
            /* clipboard denied — no-op */
        }
    };

    const handleExportData = async () => {
        setExporting(true);
        setGdprBanner(null);
        try {
            const token = localStorage.getItem('auth_token');
            const response = await fetch('/api/v1/gdpr/export', {
                headers: { Authorization: `Bearer ${token}` },
            });
            if (!response.ok) {
                const body = await response.json().catch(() => ({}));
                throw new Error(
                    body.error || body.message || `Export failed (${response.status})`,
                );
            }
            const blob = await response.blob();
            const url = URL.createObjectURL(blob);
            const a = document.createElement('a');
            const timestamp = new Date().toISOString().split('T')[0];
            a.href = url;
            a.download = `mockforge-data-export-${timestamp}.json`;
            document.body.appendChild(a);
            a.click();
            document.body.removeChild(a);
            URL.revokeObjectURL(url);
            setGdprBanner({ kind: 'success', message: 'Your data has been downloaded.' });
        } catch (err) {
            setGdprBanner({
                kind: 'error',
                message: err instanceof Error ? err.message : 'Failed to export data',
            });
        } finally {
            setExporting(false);
        }
    };

    const handleDeleteAccount = async (e: React.FormEvent) => {
        e.preventDefault();
        if (deleteConfirm !== 'DELETE') {
            setGdprBanner({
                kind: 'error',
                message: 'Type DELETE to confirm account erasure.',
            });
            return;
        }
        setDeleting(true);
        setGdprBanner(null);
        try {
            const token = localStorage.getItem('auth_token');
            const response = await fetch('/api/v1/gdpr/erase', {
                method: 'DELETE',
                headers: {
                    Authorization: `Bearer ${token}`,
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({
                    confirm: true,
                    reason: deleteReason.trim() || undefined,
                }),
            });
            if (!response.ok) {
                const body = await response.json().catch(() => ({}));
                throw new Error(
                    body.error || body.message || `Erase failed (${response.status})`,
                );
            }
            // Data erased — log out and redirect
            useAuthStore.getState().logout();
            window.location.href = '/';
        } catch (err) {
            setGdprBanner({
                kind: 'error',
                message: err instanceof Error ? err.message : 'Failed to erase account',
            });
            setDeleting(false);
        }
    };

    const handleSaveNotifications = async (
        patch: { email_notifications?: boolean; security_alerts?: boolean },
    ) => {
        setNotifSaving(true);
        setNotifBanner(null);
        try {
            const res = await authApi.updateNotifications(patch);
            setEmailNotifications(res.email_notifications);
            setSecurityAlerts(res.security_alerts);
        } catch (err) {
            setNotifBanner({
                kind: 'error',
                message: err instanceof Error ? err.message : 'Failed to save preferences',
            });
        } finally {
            setNotifSaving(false);
        }
    };

    if (!user) return null;

    return (
        <Dialog open={open} onOpenChange={onOpenChange}>
            <DialogContent className="sm:max-w-lg bg-card max-h-[90vh] overflow-y-auto">
                <DialogHeader className="space-y-2">
                    <DialogTitle className="text-xl font-semibold text-foreground">
                        Account Settings
                    </DialogTitle>
                    <DialogDescription className="text-sm text-muted-foreground leading-relaxed">
                        Manage your password, two-factor authentication, and notification preferences.
                    </DialogDescription>
                    <DialogClose onClick={() => onOpenChange(false)} />
                </DialogHeader>

                <div className="space-y-8">
                    {/* Password section */}
                    <form onSubmit={handlePasswordSubmit} className="space-y-3">
                        <div className="flex items-center gap-2 text-sm font-semibold text-foreground">
                            <Shield className="h-4 w-4" />
                            <span>Change password</span>
                        </div>

                        {pwBanner && (
                            <Banner banner={pwBanner} />
                        )}

                        <LabeledInput
                            id="currentPassword"
                            label="Current password"
                            type="password"
                            value={pwCurrent}
                            onChange={setPwCurrent}
                            error={pwErrors.current}
                        />
                        <LabeledInput
                            id="newPassword"
                            label="New password"
                            type="password"
                            value={pwNew}
                            onChange={setPwNew}
                            error={pwErrors.new}
                            placeholder="At least 8 characters"
                        />
                        <LabeledInput
                            id="confirmPassword"
                            label="Confirm new password"
                            type="password"
                            value={pwConfirm}
                            onChange={setPwConfirm}
                            error={pwErrors.confirm}
                        />
                        <div className="flex justify-end">
                            <Button type="submit" disabled={pwSubmitting}>
                                {pwSubmitting ? 'Updating…' : 'Update password'}
                            </Button>
                        </div>
                    </form>

                    {/* 2FA section */}
                    <div className="space-y-3">
                        <div className="flex items-center gap-2 text-sm font-semibold text-foreground">
                            <Lock className="h-4 w-4" />
                            <span>Two-factor authentication</span>
                        </div>

                        {twoFactorBanner && <Banner banner={twoFactorBanner} />}

                        {!twoFactorSetup && !showDisableForm && (
                            <div className="flex items-center justify-between p-3 bg-muted/50 rounded-md">
                                <div>
                                    <div className="text-sm font-medium text-foreground">
                                        {twoFactorEnabled ? '2FA is enabled' : '2FA is disabled'}
                                    </div>
                                    <div className="text-xs text-muted-foreground">
                                        {twoFactorEnabled
                                            ? 'A TOTP code is required to sign in.'
                                            : 'Require a TOTP code at sign-in for added security.'}
                                    </div>
                                </div>
                                {twoFactorEnabled ? (
                                    <Button
                                        type="button"
                                        variant="outline"
                                        onClick={() => setShowDisableForm(true)}
                                        disabled={twoFactorLoading}
                                    >
                                        Disable
                                    </Button>
                                ) : (
                                    <Button
                                        type="button"
                                        onClick={handleBeginSetup2FA}
                                        disabled={twoFactorLoading}
                                    >
                                        {twoFactorLoading ? 'Starting…' : 'Enable'}
                                    </Button>
                                )}
                            </div>
                        )}

                        {twoFactorSetup && (
                            <form onSubmit={handleVerify2FA} className="space-y-3 rounded-md border border-border p-4">
                                <p className="text-sm text-foreground">
                                    Scan this QR code with an authenticator app, then enter the 6-digit code to confirm.
                                </p>
                                <img
                                    src={twoFactorSetup.qr_code_url}
                                    alt="TOTP QR code"
                                    className="mx-auto h-40 w-40 bg-card p-2 rounded"
                                />
                                <div className="text-xs text-muted-foreground">
                                    Secret: <code className="font-mono">{twoFactorSetup.secret}</code>
                                </div>

                                <div className="rounded-md bg-warning-50 dark:bg-warning-900/20 border border-warning-200 dark:border-warning-800 p-3 text-sm">
                                    <div className="flex items-center justify-between">
                                        <strong className="text-warning-900 dark:text-warning-200">
                                            Save these backup codes
                                        </strong>
                                        <button
                                            type="button"
                                            onClick={handleCopyBackupCodes}
                                            className="text-xs inline-flex items-center gap-1 text-warning-900 dark:text-warning-200 hover:underline"
                                        >
                                            {copiedCodes ? (
                                                <>
                                                    <CheckCircle2 className="h-3.5 w-3.5" /> Copied
                                                </>
                                            ) : (
                                                <>
                                                    <Copy className="h-3.5 w-3.5" /> Copy
                                                </>
                                            )}
                                        </button>
                                    </div>
                                    <p className="text-xs mt-1 text-warning-700 dark:text-warning-300">
                                        Each code works once if you lose access to your authenticator. They won't be shown again.
                                    </p>
                                    <pre className="mt-2 text-xs font-mono grid grid-cols-2 gap-1 text-warning-900 dark:text-warning-100">
                                        {twoFactorSetup.backup_codes.map((code) => (
                                            <span key={code}>{code}</span>
                                        ))}
                                    </pre>
                                </div>

                                <LabeledInput
                                    id="totpCode"
                                    label="6-digit code"
                                    value={twoFactorCode}
                                    onChange={setTwoFactorCode}
                                    placeholder="000000"
                                    inputMode="numeric"
                                    maxLength={6}
                                />

                                <div className="flex justify-end gap-2">
                                    <Button
                                        type="button"
                                        variant="outline"
                                        onClick={handleCancelSetup2FA}
                                        disabled={twoFactorLoading}
                                    >
                                        Cancel
                                    </Button>
                                    <Button type="submit" disabled={twoFactorLoading}>
                                        {twoFactorLoading ? 'Verifying…' : 'Verify & enable'}
                                    </Button>
                                </div>
                            </form>
                        )}

                        {showDisableForm && (
                            <form onSubmit={handleDisable2FA} className="space-y-3 rounded-md border border-border p-4">
                                <p className="text-sm text-foreground">
                                    Enter your password to disable two-factor authentication.
                                </p>
                                <LabeledInput
                                    id="disablePassword"
                                    label="Password"
                                    type="password"
                                    value={disablePassword}
                                    onChange={setDisablePassword}
                                    autoFocus
                                />
                                <div className="flex justify-end gap-2">
                                    <Button
                                        type="button"
                                        variant="outline"
                                        onClick={() => {
                                            setShowDisableForm(false);
                                            setDisablePassword('');
                                            setTwoFactorBanner(null);
                                        }}
                                        disabled={twoFactorLoading}
                                    >
                                        Cancel
                                    </Button>
                                    <Button type="submit" variant="destructive" disabled={twoFactorLoading}>
                                        {twoFactorLoading ? 'Disabling…' : 'Disable 2FA'}
                                    </Button>
                                </div>
                            </form>
                        )}
                    </div>

                    {/* Notification prefs section */}
                    <div className="space-y-3">
                        <div className="flex items-center gap-2 text-sm font-semibold text-foreground">
                            <Bell className="h-4 w-4" />
                            <span>Notifications</span>
                        </div>
                        {notifBanner && <Banner banner={notifBanner} />}

                        <ToggleRow
                            icon={<Mail className="h-4 w-4 text-muted-foreground" />}
                            label="Email notifications"
                            description="Welcome messages, subscription updates, and API-token reminders."
                            checked={emailNotifications}
                            onChange={(v) => {
                                setEmailNotifications(v);
                                handleSaveNotifications({ email_notifications: v });
                            }}
                            disabled={notifSaving}
                        />
                        <ToggleRow
                            icon={<Shield className="h-4 w-4 text-muted-foreground" />}
                            label="Security alerts"
                            description="Emails when your password or 2FA is changed."
                            checked={securityAlerts}
                            onChange={(v) => {
                                setSecurityAlerts(v);
                                handleSaveNotifications({ security_alerts: v });
                            }}
                            disabled={notifSaving}
                        />
                    </div>

                    {/* GDPR / privacy section */}
                    <div className="space-y-3">
                        <div className="flex items-center gap-2 text-sm font-semibold text-foreground">
                            <Download className="h-4 w-4" />
                            <span>Privacy &amp; data</span>
                        </div>

                        {gdprBanner && <Banner banner={gdprBanner} />}

                        <div className="flex items-start justify-between gap-4 p-3 bg-muted/50 rounded-md">
                            <div>
                                <div className="text-sm font-medium text-foreground">
                                    Export your data
                                </div>
                                <div className="text-xs text-muted-foreground">
                                    Download a JSON file containing all your account data (GDPR right to data portability).
                                </div>
                            </div>
                            <Button
                                type="button"
                                variant="outline"
                                onClick={handleExportData}
                                disabled={exporting}
                            >
                                {exporting ? 'Preparing…' : 'Export'}
                            </Button>
                        </div>

                        {!showDeleteForm ? (
                            <div className="flex items-start justify-between gap-4 p-3 rounded-md border border-danger-200 dark:border-danger-800 bg-danger-50/50 dark:bg-danger-900/10">
                                <div>
                                    <div className="text-sm font-medium text-danger-900 dark:text-danger-200">
                                        Delete your account
                                    </div>
                                    <div className="text-xs text-danger-700 dark:text-danger-300">
                                        Permanently erase all your data. This cannot be undone.
                                    </div>
                                </div>
                                <Button
                                    type="button"
                                    variant="destructive"
                                    onClick={() => {
                                        setShowDeleteForm(true);
                                        setGdprBanner(null);
                                    }}
                                >
                                    <Trash2 className="h-4 w-4 mr-1" />
                                    Delete
                                </Button>
                            </div>
                        ) : (
                            <form
                                onSubmit={handleDeleteAccount}
                                className="space-y-3 rounded-md border border-danger-200 dark:border-danger-800 bg-danger-50/50 dark:bg-danger-900/10 p-4"
                            >
                                <p className="text-sm text-danger-900 dark:text-danger-200">
                                    This action is irreversible. All your workspaces, mocks, tokens, and usage data will be erased.
                                </p>
                                <LabeledInput
                                    id="deleteConfirm"
                                    label="Type DELETE to confirm"
                                    value={deleteConfirm}
                                    onChange={setDeleteConfirm}
                                    placeholder="DELETE"
                                />
                                <LabeledInput
                                    id="deleteReason"
                                    label="Reason (optional)"
                                    value={deleteReason}
                                    onChange={setDeleteReason}
                                    placeholder="Help us improve — why are you leaving?"
                                />
                                <div className="flex justify-end gap-2">
                                    <Button
                                        type="button"
                                        variant="outline"
                                        onClick={() => {
                                            setShowDeleteForm(false);
                                            setDeleteConfirm('');
                                            setDeleteReason('');
                                            setGdprBanner(null);
                                        }}
                                        disabled={deleting}
                                    >
                                        Cancel
                                    </Button>
                                    <Button type="submit" variant="destructive" disabled={deleting}>
                                        {deleting ? 'Erasing…' : 'Permanently delete account'}
                                    </Button>
                                </div>
                            </form>
                        )}
                    </div>
                </div>

                <DialogFooter>
                    <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
                        Close
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    );
}

function Banner({ banner }: { banner: NonNullable<Banner> }) {
    const className =
        banner.kind === 'success'
            ? 'text-sm text-success-700 dark:text-success-300 bg-success-100 dark:bg-success-900/30 p-3 rounded-md'
            : 'text-sm text-destructive bg-destructive/10 p-3 rounded-md';
    return <div className={className}>{banner.message}</div>;
}

interface LabeledInputProps {
    id: string;
    label: string;
    type?: string;
    value: string;
    onChange: (value: string) => void;
    placeholder?: string;
    error?: string;
    autoFocus?: boolean;
    inputMode?: 'text' | 'numeric';
    maxLength?: number;
}

function LabeledInput({
    id,
    label,
    type = 'text',
    value,
    onChange,
    placeholder,
    error,
    autoFocus,
    inputMode,
    maxLength,
}: LabeledInputProps) {
    return (
        <div className="space-y-1.5">
            <label htmlFor={id} className="text-sm font-medium text-foreground">
                {label}
            </label>
            <Input
                id={id}
                type={type}
                value={value}
                onChange={(e) => onChange(e.target.value)}
                placeholder={placeholder}
                autoFocus={autoFocus}
                inputMode={inputMode}
                maxLength={maxLength}
                className={`bg-card text-foreground ${error ? 'border-destructive' : ''}`}
            />
            {error && <p className="text-xs text-destructive">{error}</p>}
        </div>
    );
}

interface ToggleRowProps {
    icon: React.ReactNode;
    label: string;
    description: string;
    checked: boolean;
    onChange: (value: boolean) => void;
    disabled?: boolean;
}

function ToggleRow({ icon, label, description, checked, onChange, disabled }: ToggleRowProps) {
    return (
        <div className="flex items-start justify-between gap-4 p-3 bg-muted/50 rounded-md">
            <div className="flex items-start gap-2">
                {icon}
                <div>
                    <div className="text-sm font-medium text-foreground">{label}</div>
                    <div className="text-xs text-muted-foreground">{description}</div>
                </div>
            </div>
            <label className="relative inline-flex items-center cursor-pointer shrink-0">
                <input
                    type="checkbox"
                    checked={checked}
                    onChange={(e) => onChange(e.target.checked)}
                    disabled={disabled}
                    className="sr-only peer"
                />
                <div className="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-brand-300 dark:peer-focus:ring-brand-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-card after:border-border after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-brand-600" />
            </label>
        </div>
    );
}
