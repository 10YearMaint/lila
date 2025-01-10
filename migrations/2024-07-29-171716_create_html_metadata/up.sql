CREATE TABLE html_metadata (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path TEXT NOT NULL,
    html_content TEXT NOT NULL DEFAULT ''
);
