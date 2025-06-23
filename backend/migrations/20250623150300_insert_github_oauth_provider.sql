-- Insert a default GitHub OAuth2 provider configuration.
-- IMPORTANT:
-- 1. Replace 'YOUR_GITHUB_CLIENT_ID' with your actual GitHub OAuth App Client ID.
-- 2. Replace 'YOUR_ENCRYPTED_GITHUB_CLIENT_SECRET' with the AES-256-GCM encrypted version of your Client Secret.
--    You must use the same NOTIFICATION_ENCRYPTION_KEY from your .env file to encrypt it.

INSERT INTO oauth2_providers (
    provider_name,
    client_id,
    client_secret,
    auth_url,
    token_url,
    user_info_url,
    scopes,
    user_info_mapping,
    enabled,
    created_at,
    updated_at,
    icon_url
) VALUES (
    'github',
    'YOUR_GITHUB_CLIENT_ID',
    'YOUR_ENCRYPTED_GITHUB_CLIENT_SECRET',
    'https://github.com/login/oauth/authorize',
    'https://github.com/login/oauth/access_token',
    'https://api.github.com/user',
    'read:user,user:email',
    '{
        "id_field": "id",
        "email_field": "email",
        "username_field": "login"
    }',
    true,
    CURRENT_TIMESTAMP,
    CURRENT_TIMESTAMP,
    'https://github.githubassets.com/images/modules/logos_page/GitHub-Mark.png'
);