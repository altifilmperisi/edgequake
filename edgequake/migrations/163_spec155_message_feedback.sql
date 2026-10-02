-- SPEC-155 B2/B3: thumbs feedback on messages + stream finish_reason (e.g. interrupted).

ALTER TABLE messages
    ADD COLUMN IF NOT EXISTS feedback_rating VARCHAR(10),
    ADD COLUMN IF NOT EXISTS feedback_reason TEXT,
    ADD COLUMN IF NOT EXISTS finish_reason VARCHAR(50);

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'messages_valid_feedback_rating'
    ) THEN
        ALTER TABLE messages
            ADD CONSTRAINT messages_valid_feedback_rating
            CHECK (feedback_rating IS NULL OR feedback_rating IN ('up', 'down'));
    END IF;
END $$;

COMMENT ON COLUMN messages.feedback_rating IS 'User thumbs: up or down (nullable)';
COMMENT ON COLUMN messages.feedback_reason IS 'Optional free-text reason for feedback';
COMMENT ON COLUMN messages.finish_reason IS 'Assistant generation finish reason (stop, interrupted, …)';
