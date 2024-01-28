CREATE TABLE snapshot (
       id INTEGER PRIMARY KEY,
       created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
       snapshot_hash VARCHAR(64) NOT NULL,
       database_id INT NOT NULL,
       next_snapshot_id INT,
       snapshot_type SMALLINT NOT NULL,

       FOREIGN KEY(database_id) REFERENCES database(id),
       FOREIGN KEY(next_snapshot_id) REFERENCES snapshot(id),
       UNIQUE(snapshot_hash)
);
