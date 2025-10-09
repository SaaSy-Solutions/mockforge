-- Seed data for testing

-- Create admin user (password: admin123)
INSERT INTO users (id, username, email, password_hash, is_admin, is_verified)
VALUES (
    '00000000-0000-0000-0000-000000000001'::uuid,
    'admin',
    'admin@mockforge.dev',
    '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5lW7BhPx0gFKi', -- admin123
    true,
    true
);

-- Create test user (password: test123)
INSERT INTO users (id, username, email, password_hash, is_verified)
VALUES (
    '00000000-0000-0000-0000-000000000002'::uuid,
    'testuser',
    'test@example.com',
    '$2b$12$8mKE/9bqNqNqNqNqNqNqNuKqNqNqNqNqNqNqNqNqNqNqNqNqNqNq', -- test123
    true
);

-- Seed common tags
INSERT INTO tags (name) VALUES
    ('authentication'),
    ('jwt'),
    ('oauth'),
    ('security'),
    ('template'),
    ('crypto'),
    ('datasource'),
    ('database'),
    ('graphql'),
    ('testing');

-- Example plugin: auth-jwt (Official)
INSERT INTO plugins (
    id, name, description, current_version, category, license,
    repository, homepage, author_id, verified_at, downloads_total, rating_avg, rating_count
) VALUES (
    '10000000-0000-0000-0000-000000000001'::uuid,
    'auth-jwt',
    'JWT authentication plugin for MockForge. Supports RS256, HS256 algorithms with key rotation.',
    '1.2.0',
    'auth',
    'MIT',
    'https://github.com/mockforge-plugins/auth-jwt',
    'https://docs.mockforge.dev/plugins/auth-jwt',
    '00000000-0000-0000-0000-000000000001'::uuid,
    NOW(),
    1523,
    4.8,
    12
);

-- auth-jwt versions
INSERT INTO plugin_versions (plugin_id, version, download_url, checksum, file_size, downloads)
VALUES
    (
        '10000000-0000-0000-0000-000000000001'::uuid,
        '1.2.0',
        'https://registry.mockforge.dev/plugins/auth-jwt/1.2.0.wasm',
        'abc123def456',
        45678,
        1200
    ),
    (
        '10000000-0000-0000-0000-000000000001'::uuid,
        '1.1.0',
        'https://registry.mockforge.dev/plugins/auth-jwt/1.1.0.wasm',
        'def456ghi789',
        43210,
        323
    );

-- auth-jwt tags
INSERT INTO plugin_tags (plugin_id, tag_id)
SELECT '10000000-0000-0000-0000-000000000001'::uuid, id FROM tags WHERE name IN ('authentication', 'jwt', 'security');

-- Example plugin: template-crypto
INSERT INTO plugins (
    id, name, description, current_version, category, license,
    repository, author_id, verified_at, downloads_total, rating_avg, rating_count
) VALUES (
    '10000000-0000-0000-0000-000000000002'::uuid,
    'template-crypto',
    'Cryptographic template functions including hashing, encryption, and random generation.',
    '1.0.1',
    'template',
    'Apache-2.0',
    'https://github.com/mockforge-plugins/template-crypto',
    '00000000-0000-0000-0000-000000000001'::uuid,
    NOW(),
    892,
    4.5,
    8
);

INSERT INTO plugin_versions (plugin_id, version, download_url, checksum, file_size, downloads)
VALUES (
    '10000000-0000-0000-0000-000000000002'::uuid,
    '1.0.1',
    'https://registry.mockforge.dev/plugins/template-crypto/1.0.1.wasm',
    'crypto123abc',
    32456,
    892
);

INSERT INTO plugin_tags (plugin_id, tag_id)
SELECT '10000000-0000-0000-0000-000000000002'::uuid, id FROM tags WHERE name IN ('template', 'crypto', 'security');

-- Example plugin: datasource-csv
INSERT INTO plugins (
    id, name, description, current_version, category, license,
    repository, author_id, verified_at, downloads_total, rating_avg, rating_count
) VALUES (
    '10000000-0000-0000-0000-000000000003'::uuid,
    'datasource-csv',
    'CSV file data source connector with delimiter detection and schema inference.',
    '2.0.0',
    'datasource',
    'MIT',
    'https://github.com/mockforge-plugins/datasource-csv',
    '00000000-0000-0000-0000-000000000001'::uuid,
    NOW(),
    2341,
    4.9,
    15
);

INSERT INTO plugin_versions (plugin_id, version, download_url, checksum, file_size, downloads)
VALUES (
    '10000000-0000-0000-0000-000000000003'::uuid,
    '2.0.0',
    'https://registry.mockforge.dev/plugins/datasource-csv/2.0.0.wasm',
    'csv789xyz',
    51234,
    2341
);

INSERT INTO plugin_tags (plugin_id, tag_id)
SELECT '10000000-0000-0000-0000-000000000003'::uuid, id FROM tags WHERE name IN ('datasource', 'database');

-- Add some reviews
INSERT INTO reviews (plugin_id, user_id, version, rating, title, comment, helpful_count)
VALUES (
    '10000000-0000-0000-0000-000000000001'::uuid,
    '00000000-0000-0000-0000-000000000002'::uuid,
    '1.2.0',
    5,
    'Essential for authentication',
    'This plugin saved me hours of work. JWT validation works perfectly with RS256 keys. Great documentation and examples.',
    8
);

INSERT INTO reviews (plugin_id, user_id, version, rating, title, comment, helpful_count)
VALUES (
    '10000000-0000-0000-0000-000000000003'::uuid,
    '00000000-0000-0000-0000-000000000002'::uuid,
    '2.0.0',
    5,
    'CSV loading made easy',
    'Schema inference is fantastic. Handles edge cases well and performance is excellent even with large files.',
    5
);
