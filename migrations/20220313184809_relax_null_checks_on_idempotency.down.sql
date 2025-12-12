ALTER TABLE idempotency ALTER COLUMN response_status_code SET NOT NULL;
ALTER TABLE idempotency ALTER COLUMN response_body SET NOT NULL;
ALTER TABLE idempotency ALTER COLUMN response_headers SET NOT NULL;
