ALTER TABLE newsletter_issues
  DROP CONSTRAINT unique_newsletter_issues_user_id_and_slug,
  ALTER COLUMN title TYPE TEXT,
  DROP COLUMN content,
  DROP COLUMN created_at,
  DROP COLUMN description,
  DROP COLUMN published_at,
  DROP COLUMN slug,
  DROP COLUMN user_id,
  ADD COLUMN html_content TEXT NOT NULL,
  ADD COLUMN published_at TEXT NOT NULL,
  ADD COLUMN text_content TEXT NOT NULL;
