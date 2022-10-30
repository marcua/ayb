CREATE TABLE database_owner (
       id INT NOT NULL,
       slug VARCHAR(64) NOT NULL,
       
       PRIMARY KEY(id),
       UNIQUE(slug)
);       

CREATE TABLE database (
       id INT NOT NULL,
       slug VARCHAR(64) NOT NULL,
       db_type INT NOT NULL,
       owner_id INT NOT NULL,

       PRIMARY KEY(id),
       FOREIGN KEY(owner_id) REFERENCES database_owner(id),
       UNIQUE(owner_id, slug)
); 
