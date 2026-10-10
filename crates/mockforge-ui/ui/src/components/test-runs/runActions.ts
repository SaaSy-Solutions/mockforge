export function isRunActionPending(id: string, deletion: { isPending: boolean; variables?: string }, cancellation: { isPending: boolean; variables?: string }) {
    return (deletion.isPending && deletion.variables === id) || (cancellation.isPending && cancellation.variables === id);
}
