-- Fix invalid bcrypt password hashes from seed data
-- The original seed migration had a fake hash for the test user

-- Update test user (password: test123)
UPDATE users SET password_hash = '$2b$10$yG7qQ26EKTgS7h0S4YIxtOzARlcuni9IaoDmeTP88h4EXOA8I8Ho.'
WHERE id = '00000000-0000-0000-0000-000000000002'::uuid;

-- Update admin user (password: admin123)
UPDATE users SET password_hash = '$2b$10$n5RHYuZvSn5XPajDQNJMjuvfwmzzsGCFgS750U/BY1FCLBOcG9tt2'
WHERE id = '00000000-0000-0000-0000-000000000001'::uuid;
