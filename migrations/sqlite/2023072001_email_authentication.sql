CREATE TABLE authentication_method (
       id INTEGER PRIMARY KEY,
       entity_id INT NOT NULL,
       method_type SMALLINT NOT NULL,
       status SMALLINT NOT NULL,
       email_address VARCHAR(256) NOT NULL,

       FOREIGN KEY(entity_id) REFERENCES entity(id),
       UNIQUE(method_type, email_address)
); 
