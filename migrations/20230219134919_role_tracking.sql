-- Add migration script here

CREATE TABLE given_roles(
       "user"           BIGINT NOT NULL,
       "role"           BIGINT NOT NULL,
       id               BIGSERIAL PRIMARY KEY,
       CONSTRAINT fk_users
                  FOREIGN KEY ("user")
                  REFERENCES users(id)
                  ON DELETE CASCADE,
       CONSTRAINT fk_roles
                  FOREIGN KEY ("role")
                  REFERENCES roles(id)
                  ON DELETE CASCADE,
       UNIQUE("user","role")
);

CREATE VIEW given_roles_v AS
       SELECT u.guild,u."user",r."role", g."user" AS _user_id,g."role" AS _role_id,g.id as _given_role_id
       FROM given_roles AS g
       INNER JOIN users AS u ON u.id=g."user"
       INNER JOIN roles AS r ON r.id=g."role";

INSERT INTO fixup_tracking(identifier) VALUES ('given_roles');
