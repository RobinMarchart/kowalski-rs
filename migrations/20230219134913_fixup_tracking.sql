-- Add migration script here

CREATE TABLE fixup_tracking(
       identifier           TEXT UNIQUE NOT NULL,
       fixed                BOOL DEFAULT FALSE NOT NULL,
       id                   BIGSERIAL PRIMARY KEY
);
