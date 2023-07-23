CREATE EXTENSION IF NOT EXISTS citext;

CREATE TABLE authentication_method (
       id SERIAL NOT NULL,
       entity_id INT NOT NULL,
       method_type SMALLINT NOT NULL,
       status SMALLINT NOT NULL,
       email_address CITEXT NOT NULL,

       PRIMARY KEY(id),
       FOREIGN KEY(entity_id) REFERENCES entity(id),
       UNIQUE(method_type, email_address)
); 
