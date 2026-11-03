-- Keep migration 001 unchanged so existing SQLx databases can upgrade.
-- Changed passwords and operator-created accounts are intentionally retained.
DELETE FROM admin_users
WHERE password_hash = '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5GyYqJZ5q5q5q'
  AND (
    (id = '00000000-0000-0000-0000-000000000001' AND username = 'admin')
    OR (id = '00000000-0000-0000-0000-000000000002' AND username = 'viewer')
    OR (id = '00000000-0000-0000-0000-000000000003' AND username = 'editor')
  );
