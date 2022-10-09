CREATE TABLE database_owner (
       id INT NOT NULL,
       owner_name VARCHAR(64) NOT NULL,
       
       PRIMARY KEY(id),
       UNIQUE(owner_name)
);       

CREATE TABLE database (
       id INT NOT NULL,
       db_name VARCHAR(64) NOT NULL,
       db_type INT NOT NULL,
       owner_id INT NOT NULL,

       PRIMARY KEY(id),
       FOREIGN KEY(owner_id) REFERENCES database_owner(id),
       UNIQUE(db_name)
); 
