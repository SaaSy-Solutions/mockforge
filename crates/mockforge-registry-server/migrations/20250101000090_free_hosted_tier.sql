-- Apply the new Free defaults to pre-existing organizations. Preserve explicit
-- positive/unlimited custom grants and every unrelated entitlement.
UPDATE organizations
SET limits_json = jsonb_set(
        jsonb_set(
            jsonb_set(limits_json, '{hosted_mocks}', 'true'::jsonb, true),
            '{max_hosted_mocks}',
            CASE WHEN limits_json -> 'max_hosted_mocks' IS NULL
                   OR limits_json -> 'max_hosted_mocks' = '0'::jsonb
                 THEN '1'::jsonb ELSE limits_json -> 'max_hosted_mocks' END,
            true
        ),
        '{ai_tokens_per_month}',
        CASE WHEN limits_json -> 'ai_tokens_per_month' IS NULL
               OR limits_json -> 'ai_tokens_per_month' = '0'::jsonb
             THEN '5000'::jsonb ELSE limits_json -> 'ai_tokens_per_month' END,
        true
    ),
    updated_at = NOW()
WHERE plan = 'free';
