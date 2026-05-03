-- Add up migration script here
-- Add up migration script here
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    username VARCHAR(255) UNIQUE NOT NULL, -- add UNIQUE
    email VARCHAR(255) UNIQUE NOT NULL,    -- add UNIQUE
    password VARCHAR(255) NOT NULL
);