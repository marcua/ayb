CREATE TABLE entity (
       id INTEGER PRIMARY KEY,
       slug VARCHAR(64) NOT NULL,
       entity_type SMALLINT NOT NULL,
       
       UNIQUE(slug)
);       

CREATE TABLE database (
       id INTEGER PRIMARY KEY,
       slug VARCHAR(64) NOT NULL,
       db_type SMALLINT NOT NULL,
       entity_id INT NOT NULL,

       FOREIGN KEY(entity_id) REFERENCES entity(id),
       UNIQUE(entity_id, slug)
); 
