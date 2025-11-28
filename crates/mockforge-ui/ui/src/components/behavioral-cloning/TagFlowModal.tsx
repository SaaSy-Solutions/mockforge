import React, { useState } from 'react';
import { X } from 'lucide-react';
import type { Flow, TagFlowRequest } from '../../types';
import { apiService } from '../../services/api';
import { logger } from '../../utils/logger';

interface TagFlowModalProps {
  flow: Flow;
  onClose: () => void;
  onTagged: () => void;
}

export function TagFlowModal({ flow, onClose, onTagged }: TagFlowModalProps) {
  const [name, setName] = useState(flow.name || '');
  const [description, setDescription] = useState(flow.description || '');
  const [tags, setTags] = useState(flow.tags?.join(', ') || '');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError(null);

    try {
      const request: TagFlowRequest = {
        name: name.trim() || undefined,
        description: description.trim() || undefined,
        tags: tags
          .split(',')
          .map((t) => t.trim())
          .filter((t) => t.length > 0),
      };

      await apiService.tagFlow(flow.id, request);
      onTagged();
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to tag flow';
      setError(errorMessage);
      logger.error('Failed to tag flow', { error: err });
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-background rounded-lg shadow-xl w-full max-w-md p-6">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-xl font-semibold">Tag Flow</h2>
          <button
            onClick={onClose}
            className="p-1 hover:bg-muted rounded-md transition-colors"
          >
            <X className="h-5 w-5" />
          </button>
        </div>

        {error && (
          <div className="mb-4 p-3 bg-destructive/10 text-destructive rounded-md text-sm">
            {error}
          </div>
        )}

        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label className="block text-sm font-medium mb-1">Name</label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="w-full px-3 py-2 border rounded-md focus:outline-none focus:ring-2 focus:ring-primary"
              placeholder="e.g., checkout_success"
            />
          </div>

          <div>
            <label className="block text-sm font-medium mb-1">Description</label>
            <textarea
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              className="w-full px-3 py-2 border rounded-md focus:outline-none focus:ring-2 focus:ring-primary"
              rows={3}
              placeholder="Describe this flow..."
            />
          </div>

          <div>
            <label className="block text-sm font-medium mb-1">Tags (comma-separated)</label>
            <input
              type="text"
              value={tags}
              onChange={(e) => setTags(e.target.value)}
              className="w-full px-3 py-2 border rounded-md focus:outline-none focus:ring-2 focus:ring-primary"
              placeholder="e.g., ecommerce, checkout, success"
            />
          </div>

          <div className="flex gap-2 justify-end pt-4">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 text-sm font-medium text-muted-foreground hover:bg-muted rounded-md transition-colors"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={loading}
              className="px-4 py-2 text-sm font-medium bg-primary text-primary-foreground rounded-md hover:bg-primary/90 transition-colors disabled:opacity-50"
            >
              {loading ? 'Saving...' : 'Save'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

