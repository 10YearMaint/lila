CREATE TABLE html_metadata (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    file_path TEXT NOT NULL
);

CREATE TABLE html_content (
    id INTEGER NOT NULL,
    content TEXT NOT NULL,
    FOREIGN KEY (id) REFERENCES html_metadata(id) ON DELETE CASCADE
);
