ALTER TABLE entity
    ADD display_name VARCHAR(35),
    ADD description VARCHAR(100),
    ADD organization VARCHAR(35),
    ADD location VARCHAR(35),
    ADD links JSONB;
