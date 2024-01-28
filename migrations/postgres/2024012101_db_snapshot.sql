CREATE TABLE snapshot (
       id SERIAL NOT NULL,
       created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
       hash VARCHAR(64) NOT NULL,
       database_id INT NOT NULL,
       next_snapshot_id INT,
       snapshot_type SMALLINT NOT NULL,

       PRIMARY KEY (id),
       FOREIGN KEY(database_id) REFERENCES database(id) ON DELETE CASCADE,
       FOREIGN KEY(next_snapshot_id) REFERENCES snapshot(id) ON DELETE SET NULL,
       UNIQUE(hash)
);
