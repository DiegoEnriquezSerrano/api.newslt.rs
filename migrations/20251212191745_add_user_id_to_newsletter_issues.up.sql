ALTER TABLE newsletter_issues
  DROP COLUMN html_content,
  DROP COLUMN published_at,
  DROP COLUMN text_content,
  ADD COLUMN content TEXT NOT NULL,
  ADD COLUMN created_at TIMESTAMPTZ NOT NULL,
  ADD COLUMN description VARCHAR(200) NOT NULL,
  ADD COLUMN published_at TIMESTAMPTZ,
  ADD COLUMN slug VARCHAR(70) NOT NULL,
  ADD COLUMN user_id UUID NOT NULL,
  ALTER COLUMN title TYPE VARCHAR(70),
  ADD CONSTRAINT fkey_users_newsletter_issues
    FOREIGN KEY (user_id)
    REFERENCES users(user_id)
    ON UPDATE CASCADE
    ON DELETE CASCADE,
  ADD CONSTRAINT unique_newsletter_issues_user_id_and_slug
    UNIQUE (user_id, slug);
