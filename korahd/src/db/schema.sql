CREATE TABLE schema (
    version INTEGER NOT NULL
);

CREATE TABLE config (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);

-- Default values.

INSERT INTO schema(version)
VALUES (0);

INSERT INTO config(key, value)
VALUES ("llm_model", "qwen2.5");
