-- A UUID foreign key alone permits a deployment to reference another tenant's
-- project. Enforce the organization pair even for elevated/background writers.
ALTER TABLE projects ADD CONSTRAINT projects_id_org_unique UNIQUE (id, org_id);

ALTER TABLE hosted_mocks DROP CONSTRAINT hosted_mocks_project_id_fkey;
ALTER TABLE hosted_mocks ADD CONSTRAINT hosted_mocks_project_org_fkey
    FOREIGN KEY (project_id, org_id) REFERENCES projects (id, org_id)
    ON DELETE SET NULL (project_id) NOT VALID;

-- NOT VALID preserves legacy rows for an explicit ownership audit; new inserts
-- and relationship updates are enforced immediately. Project deletion must
-- clear only project_id, retaining the deployment's organization and cleanup.
