/**
 * Showcase Admin — submit / publish / feature / delete showcase entries (#12).
 *
 * The public ShowcasePage already renders is_published=true entries. This
 * admin surface is the writable counterpart so an operator can curate
 * the gallery without curl.
 */
import React, { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
    Plus,
    RefreshCw,
    Trash2,
    Star,
    StarOff,
    Eye,
    EyeOff,
    Award,
} from 'lucide-react';
import { isCloudMode } from '../utils/cloudMode';
import {
    cloudShowcaseApi,
    type ShowcaseEntry,
    type CreateShowcaseEntryRequest,
} from '../services/api/cloudShowcase';

export const CloudShowcaseAdminPage: React.FC = () => {
    if (!isCloudMode()) {
        return (
            <div className="p-6 max-w-7xl mx-auto">
                <div className="bg-blue-50 dark:bg-blue-900/20 text-blue-800 dark:text-blue-300 p-4 rounded-lg">
                    Showcase authoring only works in cloud mode — the gallery is hosted on the
                    registry.
                </div>
            </div>
        );
    }
    return <AdminView />;
};

const AdminView: React.FC = () => {
    const queryClient = useQueryClient();
    const [showCreate, setShowCreate] = useState(false);
    const [draft, setDraft] = useState<CreateShowcaseEntryRequest>({
        slug: '',
        title: '',
        description: '',
        tags: [],
    });
    const [tagsRaw, setTagsRaw] = useState('');

    // adminList returns every entry — published + unpublished — so the
    // admin can see drafts and submissions awaiting review.
    const entriesQuery = useQuery({
        queryKey: ['cloud', 'showcase', 'admin-entries'],
        queryFn: () => cloudShowcaseApi.adminList(),
    });

    const createMutation = useMutation({
        mutationFn: () =>
            cloudShowcaseApi.adminCreate({
                ...draft,
                tags: tagsRaw
                    .split(',')
                    .map((t) => t.trim())
                    .filter(Boolean),
            }),
        onSuccess: () => {
            setShowCreate(false);
            setDraft({ slug: '', title: '', description: '', tags: [] });
            setTagsRaw('');
            queryClient.invalidateQueries({ queryKey: ['cloud', 'showcase', 'admin-entries'] });
        },
    });

    const togglePublishedMutation = useMutation({
        mutationFn: ({ id, published }: { id: string; published: boolean }) =>
            cloudShowcaseApi.adminUpdate(id, { is_published: published }),
        onSuccess: () =>
            queryClient.invalidateQueries({ queryKey: ['cloud', 'showcase', 'entries'] }),
    });

    const toggleFeaturedMutation = useMutation({
        mutationFn: ({ id, featured }: { id: string; featured: boolean }) =>
            cloudShowcaseApi.adminUpdate(id, { is_featured: featured }),
        onSuccess: () =>
            queryClient.invalidateQueries({ queryKey: ['cloud', 'showcase', 'entries'] }),
    });

    const deleteMutation = useMutation({
        mutationFn: (id: string) => cloudShowcaseApi.adminDelete(id),
        onSuccess: () =>
            queryClient.invalidateQueries({ queryKey: ['cloud', 'showcase', 'entries'] }),
    });

    const entries = entriesQuery.data ?? [];

    return (
        <div className="p-6 max-w-7xl mx-auto">
            <div className="flex justify-between items-start mb-6">
                <div>
                    <h1 className="text-2xl font-bold text-gray-900 dark:text-gray-100 mb-2 flex items-center gap-2">
                        <Award className="w-6 h-6 text-amber-500" />
                        Showcase Admin
                    </h1>
                    <p className="text-gray-600 dark:text-gray-400">
                        Curate the public gallery — submit new entries, toggle publish + featured
                        status, remove obsolete ones.
                    </p>
                </div>
                <div className="flex gap-2">
                    <button
                        onClick={() => entriesQuery.refetch()}
                        className="flex items-center px-3 py-2 border border-gray-300 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-700 rounded-lg text-sm"
                    >
                        <RefreshCw
                            className={`w-4 h-4 mr-2 ${entriesQuery.isFetching ? 'animate-spin' : ''}`}
                        />
                        Refresh
                    </button>
                    <button
                        onClick={() => setShowCreate(true)}
                        className="flex items-center px-4 py-2 bg-amber-600 hover:bg-amber-700 text-white rounded-lg font-medium"
                    >
                        <Plus className="w-4 h-4 mr-2" />
                        New Entry
                    </button>
                </div>
            </div>

            {entriesQuery.isError && (
                <div className="mb-4 bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-400 p-4 rounded-lg text-sm">
                    {(entriesQuery.error as Error).message}
                </div>
            )}

            {entries.length === 0 && !entriesQuery.isLoading ? (
                <div className="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-12 text-center">
                    <Award className="w-16 h-16 mx-auto text-gray-400 mb-4" />
                    <h3 className="text-lg font-medium text-gray-900 dark:text-gray-100 mb-2">
                        Nothing published yet
                    </h3>
                    <p className="text-gray-500 dark:text-gray-400 mb-6">
                        Submit your first entry to get the gallery started.
                    </p>
                </div>
            ) : (
                <div className="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden">
                    <table className="w-full text-left text-sm">
                        <thead className="bg-gray-50 dark:bg-gray-900/50 border-b border-gray-200 dark:border-gray-700">
                            <tr>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Title</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Slug</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Tags</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Likes</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400">Featured</th>
                                <th className="px-6 py-4 font-medium text-gray-500 dark:text-gray-400 text-right">Actions</th>
                            </tr>
                        </thead>
                        <tbody className="divide-y divide-gray-200 dark:divide-gray-700">
                            {entries.map((e) => (
                                <EntryRow
                                    key={e.id}
                                    entry={e}
                                    onTogglePublished={() =>
                                        togglePublishedMutation.mutate({
                                            id: e.id,
                                            published: !e.is_published,
                                        })
                                    }
                                    onToggleFeatured={() =>
                                        toggleFeaturedMutation.mutate({
                                            id: e.id,
                                            featured: !e.is_featured,
                                        })
                                    }
                                    onDelete={() => {
                                        if (confirm(`Delete "${e.title}"? This cannot be undone.`))
                                            deleteMutation.mutate(e.id);
                                    }}
                                />
                            ))}
                        </tbody>
                    </table>
                </div>
            )}

            {showCreate && (
                <CreateModal
                    state={draft}
                    setState={setDraft}
                    tagsRaw={tagsRaw}
                    setTagsRaw={setTagsRaw}
                    onClose={() => setShowCreate(false)}
                    onSubmit={() => createMutation.mutate()}
                    submitting={createMutation.isPending}
                    error={createMutation.error ? (createMutation.error as Error).message : null}
                />
            )}
        </div>
    );
};

const EntryRow: React.FC<{
    entry: ShowcaseEntry;
    onTogglePublished: () => void;
    onToggleFeatured: () => void;
    onDelete: () => void;
}> = ({ entry, onTogglePublished, onToggleFeatured, onDelete }) => (
    <tr className="hover:bg-gray-50 dark:hover:bg-gray-800/50">
        <td className="px-6 py-4">
            <div className="font-medium text-gray-900 dark:text-gray-100">{entry.title}</div>
            <div className="text-xs text-gray-500 mt-0.5 line-clamp-1">{entry.description}</div>
        </td>
        <td className="px-6 py-4 font-mono text-xs text-gray-600 dark:text-gray-300">{entry.slug}</td>
        <td className="px-6 py-4 text-xs text-gray-600 dark:text-gray-300">
            {entry.tags.length === 0 ? (
                <span className="italic text-gray-400">none</span>
            ) : (
                entry.tags.map((t) => (
                    <span
                        key={t}
                        className="inline-block px-2 py-0.5 mr-1 mb-1 rounded-full bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-300 text-xs"
                    >
                        {t}
                    </span>
                ))
            )}
        </td>
        <td className="px-6 py-4 text-gray-600 dark:text-gray-300">{entry.likes_count}</td>
        <td className="px-6 py-4">
            {entry.is_featured ? (
                <Star className="w-4 h-4 text-amber-500 fill-amber-500" />
            ) : (
                <StarOff className="w-4 h-4 text-gray-300" />
            )}
        </td>
        <td className="px-6 py-4 text-right space-x-1">
            <button
                onClick={onTogglePublished}
                className="p-2 text-blue-600 hover:bg-blue-50 dark:hover:bg-blue-900/20 rounded-lg"
                title={entry.is_published ? 'Unpublish' : 'Publish'}
            >
                {entry.is_published ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
            </button>
            <button
                onClick={onToggleFeatured}
                className="p-2 text-amber-600 hover:bg-amber-50 dark:hover:bg-amber-900/20 rounded-lg"
                title={entry.is_featured ? 'Unfeature' : 'Feature'}
            >
                {entry.is_featured ? <StarOff className="w-4 h-4" /> : <Star className="w-4 h-4" />}
            </button>
            <button
                onClick={onDelete}
                className="p-2 text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-lg"
                title="Delete"
            >
                <Trash2 className="w-4 h-4" />
            </button>
        </td>
    </tr>
);

const CreateModal: React.FC<{
    state: CreateShowcaseEntryRequest;
    setState: React.Dispatch<React.SetStateAction<CreateShowcaseEntryRequest>>;
    tagsRaw: string;
    setTagsRaw: (s: string) => void;
    onClose: () => void;
    onSubmit: () => void;
    submitting: boolean;
    error: string | null;
}> = ({ state, setState, tagsRaw, setTagsRaw, onClose, onSubmit, submitting, error }) => (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm">
        <div className="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-2xl w-full max-h-[85vh] overflow-y-auto border border-gray-200 dark:border-gray-700">
            <div className="p-6 border-b border-gray-200 dark:border-gray-700 sticky top-0 bg-white dark:bg-gray-800">
                <h2 className="text-xl font-semibold flex items-center gap-2">
                    <Award className="w-5 h-5 text-amber-500" />
                    Submit Showcase Entry
                </h2>
                <p className="text-xs text-gray-500 mt-1">
                    Lands as <code>is_published=false</code>. Toggle publish from the row actions
                    after review.
                </p>
            </div>
            <div className="p-6 space-y-4">
                {error && (
                    <div className="bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-400 p-3 rounded text-sm">
                        {error}
                    </div>
                )}
                <div className="grid grid-cols-2 gap-3">
                    <div className="space-y-2">
                        <label className="block text-sm font-medium">Slug</label>
                        <input
                            type="text"
                            value={state.slug}
                            onChange={(e) => setState({ ...state, slug: e.target.value })}
                            placeholder="kebab-case-slug"
                            className="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg outline-none focus:ring-2 focus:ring-amber-500 font-mono text-xs"
                        />
                    </div>
                    <div className="space-y-2">
                        <label className="block text-sm font-medium">Title</label>
                        <input
                            type="text"
                            value={state.title}
                            onChange={(e) => setState({ ...state, title: e.target.value })}
                            className="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg outline-none focus:ring-2 focus:ring-amber-500"
                        />
                    </div>
                </div>
                <div className="space-y-2">
                    <label className="block text-sm font-medium">Description</label>
                    <textarea
                        value={state.description}
                        onChange={(e) => setState({ ...state, description: e.target.value })}
                        rows={3}
                        className="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg outline-none focus:ring-2 focus:ring-amber-500"
                    />
                </div>
                <div className="space-y-2">
                    <label className="block text-sm font-medium">
                        Body (markdown, optional)
                    </label>
                    <textarea
                        value={state.body ?? ''}
                        onChange={(e) => setState({ ...state, body: e.target.value })}
                        rows={5}
                        className="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg outline-none focus:ring-2 focus:ring-amber-500 font-mono text-xs"
                    />
                </div>
                <div className="grid grid-cols-2 gap-3">
                    <div className="space-y-2">
                        <label className="block text-sm font-medium">Demo URL (optional)</label>
                        <input
                            type="url"
                            value={state.demo_url ?? ''}
                            onChange={(e) => setState({ ...state, demo_url: e.target.value })}
                            className="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg outline-none focus:ring-2 focus:ring-amber-500 font-mono text-xs"
                        />
                    </div>
                    <div className="space-y-2">
                        <label className="block text-sm font-medium">Source URL (optional)</label>
                        <input
                            type="url"
                            value={state.source_url ?? ''}
                            onChange={(e) => setState({ ...state, source_url: e.target.value })}
                            className="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg outline-none focus:ring-2 focus:ring-amber-500 font-mono text-xs"
                        />
                    </div>
                </div>
                <div className="space-y-2">
                    <label className="block text-sm font-medium">Tags (comma-separated)</label>
                    <input
                        type="text"
                        value={tagsRaw}
                        onChange={(e) => setTagsRaw(e.target.value)}
                        placeholder="e.g., chaos, observability, recipe"
                        className="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-600 rounded-lg outline-none focus:ring-2 focus:ring-amber-500"
                    />
                </div>
            </div>
            <div className="p-6 border-t border-gray-200 dark:border-gray-700 flex justify-end gap-3 sticky bottom-0 bg-white dark:bg-gray-800">
                <button onClick={onClose} className="px-4 py-2">
                    Cancel
                </button>
                <button
                    onClick={onSubmit}
                    disabled={!state.slug || !state.title || !state.description || submitting}
                    className="px-4 py-2 bg-amber-600 hover:bg-amber-700 text-white rounded-lg disabled:opacity-50"
                >
                    {submitting ? 'Submitting…' : 'Submit'}
                </button>
            </div>
        </div>
    </div>
);
