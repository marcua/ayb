CREATE TABLE api_token (
       short_token VARCHAR(8) PRIMARY KEY,
       entity_id INT NOT NULL,
       hash VARCHAR(64) NOT NULL,
       status SMALLINT NOT NULL,

       FOREIGN KEY(entity_id) REFERENCES entity(id)
);
