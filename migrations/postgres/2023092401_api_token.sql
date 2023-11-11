CREATE TABLE api_token (
       short_token VARCHAR(12),
       entity_id INT NOT NULL,
       hash VARCHAR(64) NOT NULL,
       status SMALLINT NOT NULL,

       PRIMARY KEY(short_token),
       FOREIGN KEY(entity_id) REFERENCES entity(id)
);
