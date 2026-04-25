import React, { useState } from 'react';
import { CheckCircle2, XCircle, Clock, AlertTriangle } from 'lucide-react';
import { toast } from 'sonner';
import { Card, CardContent } from '../ui/Card';
import { Button } from '../ui/button';
import { Badge } from '../ui/Badge';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../ui/Dialog';
import { Label } from '../ui/label';
import { Textarea } from '../ui/textarea';
import {
  useApprovePromotion,
  usePromotions,
  useRejectPromotion,
} from '../../hooks/api/usePromotions';
import type { ScenarioPromotion } from '../../services/api/promotions';

interface Props {
  workspaceId: string;
}

const statusBadge = (status: string) => {
  switch (status.toLowerCase()) {
    case 'pending':
      return (
        <Badge variant="outline" className="gap-1">
          <Clock className="w-3 h-3" />
          Pending
        </Badge>
      );
    case 'approved':
    case 'completed':
      return (
        <Badge variant="secondary" className="gap-1">
          <CheckCircle2 className="w-3 h-3" />
          {status}
        </Badge>
      );
    case 'rejected':
    case 'failed':
      return (
        <Badge variant="destructive" className="gap-1">
          <XCircle className="w-3 h-3" />
          {status}
        </Badge>
      );
    default:
      return <Badge variant="outline">{status}</Badge>;
  }
};

const WorkspacePromotions: React.FC<Props> = ({ workspaceId }) => {
  const { data: promotions, isLoading, error } = usePromotions(workspaceId);
  const approve = useApprovePromotion(workspaceId);
  const reject = useRejectPromotion(workspaceId);

  const [approvingId, setApprovingId] = useState<string | null>(null);
  const [rejectingId, setRejectingId] = useState<string | null>(null);
  const [approvalComments, setApprovalComments] = useState('');
  const [rejectionReason, setRejectionReason] = useState('');

  const handleApprove = async (promotion: ScenarioPromotion) => {
    try {
      await approve.mutateAsync({
        promotionId: promotion.id,
        request: { comments: approvalComments.trim() || undefined },
      });
      toast.success('Promotion approved');
      setApprovingId(null);
      setApprovalComments('');
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'Failed to approve');
    }
  };

  const handleReject = async (promotion: ScenarioPromotion) => {
    if (!rejectionReason.trim()) {
      toast.error('A reason is required to reject a promotion');
      return;
    }
    try {
      await reject.mutateAsync({
        promotionId: promotion.id,
        request: { reason: rejectionReason.trim() },
      });
      toast.success('Promotion rejected');
      setRejectingId(null);
      setRejectionReason('');
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'Failed to reject');
    }
  };

  if (isLoading) {
    return <p className="text-muted-foreground">Loading promotions…</p>;
  }

  if (error) {
    return (
      <Card>
        <CardContent className="p-4 flex items-start gap-2">
          <AlertTriangle className="w-4 h-4 mt-0.5 text-destructive" />
          <div>
            <p className="font-medium">Could not load promotions</p>
            <p className="text-sm text-muted-foreground">
              {error instanceof Error ? error.message : 'Unknown error'}
            </p>
          </div>
        </CardContent>
      </Card>
    );
  }

  const activePromotion = promotions?.find(
    (p) => p.id === approvingId || p.id === rejectingId
  );

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <h3 className="text-lg font-semibold">Scenario Promotions</h3>
        {promotions && (
          <span className="text-sm text-muted-foreground">
            {promotions.length} total
          </span>
        )}
      </div>

      {!promotions || promotions.length === 0 ? (
        <p className="text-muted-foreground text-sm">
          No scenario promotions yet. Use the CLI or API to promote a scenario between environments.
        </p>
      ) : (
        <div className="space-y-2">
          {promotions.map((p) => {
            const isPending = p.status.toLowerCase() === 'pending';
            return (
              <Card key={p.id}>
                <CardContent className="p-4 space-y-2">
                  <div className="flex items-center justify-between gap-4">
                    <div className="flex items-center gap-2 flex-wrap">
                      <Badge variant="outline">{p.from_environment}</Badge>
                      <span className="text-muted-foreground">→</span>
                      <Badge variant="outline">{p.to_environment}</Badge>
                      {statusBadge(p.status)}
                      {p.requires_approval && isPending && (
                        <Badge variant="secondary" className="gap-1">
                          <AlertTriangle className="w-3 h-3" />
                          Needs approval
                        </Badge>
                      )}
                    </div>
                    {isPending && p.requires_approval && (
                      <div className="flex gap-2 shrink-0">
                        <Button size="sm" variant="outline" onClick={() => setApprovingId(p.id)}>
                          Approve
                        </Button>
                        <Button size="sm" variant="destructive" onClick={() => setRejectingId(p.id)}>
                          Reject
                        </Button>
                      </div>
                    )}
                  </div>
                  <div className="text-sm text-muted-foreground">
                    Scenario <span className="font-mono">{p.scenario_id.slice(0, 8)}</span> · v
                    {p.scenario_version}
                  </div>
                  {p.approval_required_reason && (
                    <div className="text-sm">
                      <span className="font-medium">Approval reason: </span>
                      {p.approval_required_reason}
                    </div>
                  )}
                  {p.comments && (
                    <div className="text-sm">
                      <span className="font-medium">Comments: </span>
                      {p.comments}
                    </div>
                  )}
                  {p.approval_comments && (
                    <div className="text-sm">
                      <span className="font-medium">Approver notes: </span>
                      {p.approval_comments}
                    </div>
                  )}
                  <div className="text-xs text-muted-foreground">
                    Created {new Date(p.created_at).toLocaleString()}
                    {p.completed_at ? ` · completed ${new Date(p.completed_at).toLocaleString()}` : ''}
                  </div>
                </CardContent>
              </Card>
            );
          })}
        </div>
      )}

      <Dialog
        open={!!approvingId}
        onOpenChange={(open) => {
          if (!open) {
            setApprovingId(null);
            setApprovalComments('');
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Approve Promotion</DialogTitle>
            <DialogDescription>
              Approving will complete this promotion and pin the scenario version to the target environment.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-2">
            <Label htmlFor="approval-comments">Comments (optional)</Label>
            <Textarea
              id="approval-comments"
              value={approvalComments}
              onChange={(e) => setApprovalComments(e.target.value)}
              placeholder="Why are you approving this?"
              rows={4}
            />
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setApprovingId(null)}>
              Cancel
            </Button>
            <Button
              onClick={() => activePromotion && handleApprove(activePromotion)}
              disabled={approve.isPending}
            >
              {approve.isPending ? 'Approving…' : 'Approve'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog
        open={!!rejectingId}
        onOpenChange={(open) => {
          if (!open) {
            setRejectingId(null);
            setRejectionReason('');
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Reject Promotion</DialogTitle>
            <DialogDescription>A reason is required. The promoter will see it.</DialogDescription>
          </DialogHeader>
          <div className="space-y-2">
            <Label htmlFor="rejection-reason">Reason</Label>
            <Textarea
              id="rejection-reason"
              value={rejectionReason}
              onChange={(e) => setRejectionReason(e.target.value)}
              placeholder="Why are you rejecting this?"
              rows={4}
            />
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setRejectingId(null)}>
              Cancel
            </Button>
            <Button
              variant="destructive"
              onClick={() => activePromotion && handleReject(activePromotion)}
              disabled={reject.isPending || !rejectionReason.trim()}
            >
              {reject.isPending ? 'Rejecting…' : 'Reject'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
};

export default WorkspacePromotions;
