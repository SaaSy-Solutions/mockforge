import React, { useState } from 'react';
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
import { Shield, Bell, Lock, Mail } from 'lucide-react';

interface AccountSettingsProps {
    open: boolean;
    onOpenChange: (open: boolean) => void;
}

export function AccountSettings({ open, onOpenChange }: AccountSettingsProps) {
    const { user } = useAuthStore();
    const [formData, setFormData] = useState({
        currentPassword: '',
        newPassword: '',
        confirmPassword: '',
        twoFactorEnabled: false,
        emailNotifications: true,
        securityAlerts: true,
    });
    const [isSubmitting, setIsSubmitting] = useState(false);
    const [errors, setErrors] = useState<Record<string, string>>({});
    const [successMessage, setSuccessMessage] = useState('');

    // Reset form when modal opens
    React.useEffect(() => {
        if (open) {
            setFormData({
                currentPassword: '',
                newPassword: '',
                confirmPassword: '',
                twoFactorEnabled: false,
                emailNotifications: true,
                securityAlerts: true,
            });
            setErrors({});
            setSuccessMessage('');
        }
    }, [open]);

    const validateForm = () => {
        const newErrors: Record<string, string> = {};

        if (formData.newPassword) {
            if (!formData.currentPassword) {
                newErrors.currentPassword = 'Current password is required to set a new password';
            }
            if (formData.newPassword.length < 8) {
                newErrors.newPassword = 'Password must be at least 8 characters';
            }
            if (formData.newPassword !== formData.confirmPassword) {
                newErrors.confirmPassword = 'Passwords do not match';
            }
        }

        setErrors(newErrors);
        return Object.keys(newErrors).length === 0;
    };

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault();

        if (!validateForm() || !user) return;

        setIsSubmitting(true);
        setSuccessMessage('');

        try {
            // In a real app, this would make an API call to update security settings
            // For now, we'll simulate a successful update
            await new Promise(resolve => setTimeout(resolve, 1000));

            setSuccessMessage('Account settings updated successfully');

            // Clear password fields after successful update
            setFormData(prev => ({
                ...prev,
                currentPassword: '',
                newPassword: '',
                confirmPassword: '',
            }));
        } catch {
            setErrors({ general: 'Failed to update account settings. Please try again.' });
        } finally {
            setIsSubmitting(false);
        }
    };

    const handleInputChange = (field: string, value: string | boolean) => {
        setFormData(prev => ({ ...prev, [field]: value }));
        // Clear error when user starts typing
        if (errors[field]) {
            setErrors(prev => ({ ...prev, [field]: '' }));
        }
        setSuccessMessage('');
    };

    if (!user) return null;

    return (
        <Dialog open={open} onOpenChange={onOpenChange}>
            <DialogContent className="sm:max-w-lg bg-white dark:bg-gray-900">
                <DialogHeader className="space-y-2">
                    <DialogTitle className="text-xl font-semibold text-gray-900 dark:text-gray-100">Account Settings</DialogTitle>
                    <DialogDescription className="text-sm text-gray-600 dark:text-gray-400 leading-relaxed">
                        Manage your account security and notification preferences.
                    </DialogDescription>
                    <DialogClose onClick={() => onOpenChange(false)} />
                </DialogHeader>

                <form onSubmit={handleSubmit} className="space-y-6">
                    {errors.general && (
                        <div className="text-sm text-destructive bg-destructive/10 p-3 rounded-md">
                            {errors.general}
                        </div>
                    )}

                    {successMessage && (
                        <div className="text-sm text-green-700 dark:text-green-300 bg-green-100 dark:bg-green-900/30 p-3 rounded-md">
                            {successMessage}
                        </div>
                    )}

                    {/* Security Section */}
                    <div className="space-y-4">
                        <div className="flex items-center gap-2 text-sm font-semibold text-gray-900 dark:text-gray-100">
                            <Shield className="h-4 w-4" />
                            <span>Security</span>
                        </div>

                        <div className="space-y-2">
                            <label htmlFor="currentPassword" className="text-sm font-medium text-gray-900 dark:text-gray-100">
                                Current Password
                            </label>
                            <Input
                                id="currentPassword"
                                type="password"
                                value={formData.currentPassword}
                                onChange={(e) => handleInputChange('currentPassword', e.target.value)}
                                placeholder="Enter current password"
                                className={`bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 placeholder:text-gray-500 dark:placeholder:text-gray-400 ${errors.currentPassword ? 'border-destructive' : ''}`}
                            />
                            {errors.currentPassword && (
                                <p className="text-sm text-destructive">{errors.currentPassword}</p>
                            )}
                        </div>

                        <div className="space-y-2">
                            <label htmlFor="newPassword" className="text-sm font-medium text-gray-900 dark:text-gray-100">
                                New Password
                            </label>
                            <Input
                                id="newPassword"
                                type="password"
                                value={formData.newPassword}
                                onChange={(e) => handleInputChange('newPassword', e.target.value)}
                                placeholder="Enter new password (min 8 characters)"
                                className={`bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 placeholder:text-gray-500 dark:placeholder:text-gray-400 ${errors.newPassword ? 'border-destructive' : ''}`}
                            />
                            {errors.newPassword && (
                                <p className="text-sm text-destructive">{errors.newPassword}</p>
                            )}
                        </div>

                        <div className="space-y-2">
                            <label htmlFor="confirmPassword" className="text-sm font-medium text-gray-900 dark:text-gray-100">
                                Confirm New Password
                            </label>
                            <Input
                                id="confirmPassword"
                                type="password"
                                value={formData.confirmPassword}
                                onChange={(e) => handleInputChange('confirmPassword', e.target.value)}
                                placeholder="Confirm new password"
                                className={`bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 placeholder:text-gray-500 dark:placeholder:text-gray-400 ${errors.confirmPassword ? 'border-destructive' : ''}`}
                            />
                            {errors.confirmPassword && (
                                <p className="text-sm text-destructive">{errors.confirmPassword}</p>
                            )}
                        </div>

                        <div className="flex items-center justify-between p-3 bg-gray-50 dark:bg-gray-800/50 rounded-md">
                            <div className="flex items-center gap-2">
                                <Lock className="h-4 w-4 text-gray-600 dark:text-gray-400" />
                                <span className="text-sm font-medium text-gray-900 dark:text-gray-100">Two-Factor Authentication</span>
                            </div>
                            <label className="relative inline-flex items-center cursor-pointer">
                                <input
                                    type="checkbox"
                                    checked={formData.twoFactorEnabled}
                                    onChange={(e) => handleInputChange('twoFactorEnabled', e.target.checked)}
                                    className="sr-only peer"
                                />
                                <div className="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-brand-300 dark:peer-focus:ring-brand-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full rtl:peer-checked:after:-translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-brand-600"></div>
                            </label>
                        </div>
                    </div>

                    {/* Notifications Section */}
                    <div className="space-y-4">
                        <div className="flex items-center gap-2 text-sm font-semibold text-gray-900 dark:text-gray-100">
                            <Bell className="h-4 w-4" />
                            <span>Notifications</span>
                        </div>

                        <div className="flex items-center justify-between p-3 bg-gray-50 dark:bg-gray-800/50 rounded-md">
                            <div className="flex items-center gap-2">
                                <Mail className="h-4 w-4 text-gray-600 dark:text-gray-400" />
                                <span className="text-sm font-medium text-gray-900 dark:text-gray-100">Email Notifications</span>
                            </div>
                            <label className="relative inline-flex items-center cursor-pointer">
                                <input
                                    type="checkbox"
                                    checked={formData.emailNotifications}
                                    onChange={(e) => handleInputChange('emailNotifications', e.target.checked)}
                                    className="sr-only peer"
                                />
                                <div className="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-brand-300 dark:peer-focus:ring-brand-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full rtl:peer-checked:after:-translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-brand-600"></div>
                            </label>
                        </div>

                        <div className="flex items-center justify-between p-3 bg-gray-50 dark:bg-gray-800/50 rounded-md">
                            <div className="flex items-center gap-2">
                                <Shield className="h-4 w-4 text-gray-600 dark:text-gray-400" />
                                <span className="text-sm font-medium text-gray-900 dark:text-gray-100">Security Alerts</span>
                            </div>
                            <label className="relative inline-flex items-center cursor-pointer">
                                <input
                                    type="checkbox"
                                    checked={formData.securityAlerts}
                                    onChange={(e) => handleInputChange('securityAlerts', e.target.checked)}
                                    className="sr-only peer"
                                />
                                <div className="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-brand-300 dark:peer-focus:ring-brand-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full rtl:peer-checked:after:-translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-brand-600"></div>
                            </label>
                        </div>
                    </div>
                </form>

                <DialogFooter>
                    <Button
                        type="button"
                        variant="outline"
                        onClick={() => onOpenChange(false)}
                        disabled={isSubmitting}
                    >
                        Cancel
                    </Button>
                    <Button
                        type="submit"
                        onClick={handleSubmit}
                        disabled={isSubmitting}
                    >
                        {isSubmitting ? 'Saving...' : 'Save Changes'}
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    );
}
