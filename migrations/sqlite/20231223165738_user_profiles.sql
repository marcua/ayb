ALTER TABLE entity ADD display_name VARCHAR(35);
ALTER TABLE entity ADD description VARCHAR(100);
ALTER TABLE entity ADD organization VARCHAR(35);
ALTER TABLE entity ADD location VARCHAR(35);
ALTER TABLE entity ADD links JSONB;
