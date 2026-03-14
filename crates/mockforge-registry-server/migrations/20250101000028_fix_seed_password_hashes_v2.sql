-- Fix seed password hashes with properly generated bcrypt hashes
-- Previous migration (0027) had AI-fabricated hashes that don't actually verify

-- Update admin user (password: admin123)
UPDATE users SET password_hash = '$2b$10$5mcGb.1oYTuLLi9h08MucO1rsfWmJsryFoKtHF6.qv9p5Ge6V9IW6'
WHERE id = '00000000-0000-0000-0000-000000000001'::uuid;

-- Update test user (password: test123)
UPDATE users SET password_hash = '$2b$10$Uv30udbt8TccWoXbl4r3tufv569B/XxQ2d/SsR.9Fdzjw1/ZroEAi'
WHERE id = '00000000-0000-0000-0000-000000000002'::uuid;
