CREATE TABLE entity (
       id SERIAL NOT NULL,
       slug VARCHAR(64) NOT NULL,
       entity_type SMALLINT NOT NULL,
       
       PRIMARY KEY(id),
       UNIQUE(slug)
);       

CREATE TABLE database (
       id SERIAL NOT NULL,
       slug VARCHAR(64) NOT NULL,
       db_type SMALLINT NOT NULL,
       entity_id INT NOT NULL,

       PRIMARY KEY(id),
       FOREIGN KEY(entity_id) REFERENCES entity(id),
       UNIQUE(entity_id, slug)
); 
