CREATE TABLE entity_database_permission (
       entity_id INT NOT NULL,
       database_id INT NOT NULL,       
       sharing_level SMALLINT NOT NULL,

       FOREIGN KEY(entity_id) REFERENCES entity(id),
       FOREIGN KEY(database_id) REFERENCES database(id)
       UNIQUE(entity_id, database_id)
);

ALTER TABLE database ADD public_sharing_level SMALLINT NOT NULL DEFAULT 0; -- defaults to no public access
