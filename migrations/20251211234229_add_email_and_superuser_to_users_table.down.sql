ALTER TABLE users
    DROP COLUMN email,
    DROP COLUMN is_superuser,
    ALTER COLUMN username TYPE TEXT,
    ALTER COLUMN username SET NOT NULL;
