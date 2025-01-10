CREATE TABLE html_metadata (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path TEXT NOT NULL
);

CREATE TABLE html_content (
    id INTEGER PRIMARY KEY,
    content TEXT NOT NULL,
    FOREIGN KEY(id) REFERENCES html_metadata(id) ON DELETE CASCADE
);
