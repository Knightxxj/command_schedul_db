CREATE TABLE IF NOT EXISTS tasks (
    id         TEXT PRIMARY KEY,
    content    TEXT NOT NULL,
    tag        TEXT NOT NULL DEFAULT '',
    priority   TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS id_counter (
    id      INTEGER PRIMARY KEY DEFAULT 1 CHECK (id = 1),
    last_id TEXT NOT NULL DEFAULT 'task_00000'
);

INSERT INTO id_counter (id, last_id)
VALUES (1, 'task_00000')
ON CONFLICT (id) DO NOTHING;
