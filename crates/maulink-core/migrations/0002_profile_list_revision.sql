CREATE TABLE app_metadata (
    key TEXT PRIMARY KEY,
    integer_value INTEGER NOT NULL
);

INSERT INTO app_metadata (key, integer_value)
VALUES ('profile_list_revision', 1);
