-- Free HTTP requests share one monthly funding pool across an owner's orgs.
-- Historical usage survives org/account deletion without retaining user data.
CREATE TABLE free_hosted_request_periods (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_user_id UUID NOT NULL,
    period_start DATE NOT NULL,
    granted_requests BIGINT NOT NULL CHECK(granted_requests >= -1),
    requests BIGINT NOT NULL DEFAULT 0 CHECK(requests >= 0),
    UNIQUE(owner_user_id,period_start)
);
WITH configured AS (
    SELECT o.owner_id,CASE WHEN jsonb_typeof(s.setting_value)='object'
        AND s.setting_value ? 'requests_per_30d' THEN s.setting_value->'requests_per_30d'
        ELSE o.limits_json->'requests_per_30d' END AS quota
    FROM organizations o LEFT JOIN org_settings s ON s.org_id=o.id AND s.setting_key='quota'
    WHERE o.plan='free'
), numeric_grants AS (
    SELECT owner_id,CASE WHEN jsonb_typeof(quota)='number' AND (quota #>> '{}') ~ '^-?[0-9]+$'
        THEN (quota #>> '{}')::bigint ELSE 10000 END AS quota FROM configured
), grants AS (
    SELECT owner_id,CASE WHEN BOOL_OR(quota=-1) THEN -1 ELSE GREATEST(10000,MAX(quota)) END AS quota
    FROM numeric_grants GROUP BY owner_id
)
INSERT INTO free_hosted_request_periods(owner_user_id,period_start,granted_requests,requests)
SELECT o.owner_id,u.period_start,g.quota,SUM(u.requests)
FROM usage_counters u JOIN organizations o ON o.id=u.org_id JOIN grants g ON g.owner_id=o.owner_id
WHERE o.plan='free' GROUP BY o.owner_id,u.period_start,g.quota;
ALTER TABLE free_hosted_request_periods ENABLE ROW LEVEL SECURITY;
ALTER TABLE free_hosted_request_periods FORCE ROW LEVEL SECURITY;
CREATE POLICY owned_request_allowance ON free_hosted_request_periods
USING(EXISTS(SELECT 1 FROM organizations o WHERE o.id=nullif(current_setting('app.current_org_id',true),'')::uuid AND o.owner_id=owner_user_id));
