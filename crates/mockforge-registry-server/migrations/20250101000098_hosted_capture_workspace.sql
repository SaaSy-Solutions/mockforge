-- Explicit deployment attribution for hosted captures; existing deployments
-- and captures stay unassigned until an operator chooses a workspace.
CREATE UNIQUE INDEX workspaces_id_org_id_unique ON workspaces(id, org_id);
ALTER TABLE hosted_mocks ADD COLUMN workspace_id UUID
    REFERENCES workspaces(id) ON DELETE SET NULL;
ALTER TABLE hosted_mocks ADD CONSTRAINT hosted_mocks_workspace_org_fk
    FOREIGN KEY (workspace_id, org_id) REFERENCES workspaces(id, org_id)
    DEFERRABLE INITIALLY DEFERRED;
CREATE INDEX hosted_mocks_workspace_idx ON hosted_mocks(workspace_id)
    WHERE workspace_id IS NOT NULL;
