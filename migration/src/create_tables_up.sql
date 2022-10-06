CREATE TABLE IF NOT EXISTS guilds (
    guild           BIGINT PRIMARY KEY
);

CREATE TABLE IF NOT EXISTS users (
       guild           BIGINT,
       "user"        BIGINT,
       PRIMARY KEY (guild, "user"),
       CONSTRAINT fk_guilds
                  FOREIGN KEY (guild)
                  REFERENCES guilds(guild)
                  ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS channels (
       guild           BIGINT,
       channel         BIGINT,
       PRIMARY KEY (guild, channel),
       CONSTRAINT fk_guilds
                  FOREIGN KEY (guild)
                  REFERENCES guilds(guild)
                  ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS roles (
             guild           BIGINT,
             role            BIGINT,
             PRIMARY KEY (guild, role),
             CONSTRAINT fk_guilds
                        FOREIGN KEY (guild)
                        REFERENCES guilds(guild)
                        ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS messages (
             guild           BIGINT,
             channel         BIGINT,
             message         BIGINT,
             PRIMARY KEY (guild, channel, message),
             CONSTRAINT fk_guilds
                        FOREIGN KEY (guild)
                        REFERENCES guilds(guild)
                        ON DELETE CASCADE,
             CONSTRAINT fk_channels
                        FOREIGN KEY (guild, channel)
                        REFERENCES channels(guild, channel)
                        ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS emojis (
             id              SERIAL PRIMARY KEY,
             unicode         TEXT,
             guild           BIGINT,
             guild_emoji     BIGINT,
             CONSTRAINT fk_guilds
                        FOREIGN KEY (guild)
                        REFERENCES guilds(guild)
                        ON DELETE CASCADE,
             CONSTRAINT unicode_or_guild
                        CHECK ((guild IS NULL) = (guild_emoji IS NULL)
                        AND (unicode IS NULL) != (guild_emoji IS NULL))
);

CREATE TABLE IF NOT EXISTS modules (
             guild           BIGINT PRIMARY KEY,
             status          BIT(8) NOT NULL,
             CONSTRAINT fk_guilds
                        FOREIGN KEY (guild)
                        REFERENCES guilds(guild)
                        ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS publishing (
             id              TEXT PRIMARY KEY,
             guild           BIGINT UNIQUE,
             CONSTRAINT fk_guilds
                        FOREIGN KEY (guild)
                        REFERENCES guilds(guild)
                        ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS score_auto_delete (
             guild           BIGINT PRIMARY KEY,
             score           BIGINT NOT NULL,
             CONSTRAINT fk_guilds
                        FOREIGN KEY (guild)
                            REFERENCES guilds(guild)
                            ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS score_auto_pin (
             guild           BIGINT PRIMARY KEY,
             score           BIGINT NOT NULL,
             CONSTRAINT fk_guilds
                        FOREIGN KEY (guild)
                        REFERENCES guilds(guild)
                        ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS score_cooldowns (
             guild           BIGINT,
             role            BIGINT,
             cooldown        BIGINT NOT NULL,
             PRIMARY KEY (guild, role),
             CONSTRAINT fk_guilds
                        FOREIGN KEY (guild)
                        REFERENCES guilds(guild)
                        ON DELETE CASCADE,
             CONSTRAINT fk_roles
                        FOREIGN KEY (guild, role)
                        REFERENCES roles(guild, role)
                        ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS score_drops (
             guild           BIGINT,
             channel         BIGINT,
             PRIMARY KEY (guild, channel),
             CONSTRAINT fk_guilds
                        FOREIGN KEY (guild)
                        REFERENCES guilds(guild)
                        ON DELETE CASCADE,
             CONSTRAINT fk_channels
                        FOREIGN KEY (guild, channel)
                        REFERENCES channels(guild, channel)
                        ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS score_emojis (
             guild           BIGINT,
             emoji           INT,
             upvote          BOOLEAN NOT NULL,
             PRIMARY KEY (guild, emoji),
             CONSTRAINT fk_guilds
                        FOREIGN KEY (guild)
                        REFERENCES guilds(guild)
                        ON DELETE CASCADE,
             CONSTRAINT fk_emojis
                        FOREIGN KEY (emoji)
                        REFERENCES emojis(id)
                        ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS score_reactions (
             guild           BIGINT,
             user_from       BIGINT,
             user_to         BIGINT,
             channel         BIGINT,
             message         BIGINT,
             emoji           INT,
             native          BOOLEAN NOT NULL DEFAULT true,
             PRIMARY KEY (guild, user_from, user_to, channel, message, emoji),
             CONSTRAINT fk_guilds
                        FOREIGN KEY (guild)
                        REFERENCES guilds(guild)
                        ON DELETE CASCADE,
             CONSTRAINT fk_users
                        FOREIGN KEY (guild, user_to)
                        REFERENCES users(guild, "user")
                        ON DELETE CASCADE,
             CONSTRAINT fk_score_emojis
                        FOREIGN KEY (guild, emoji)
                        REFERENCES score_emojis(guild, emoji)
                        ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS score_roles (
             guild           BIGINT,
             role            BIGINT,
             score           BIGINT,
             PRIMARY KEY (guild, role, score),
             CONSTRAINT fk_guilds
                        FOREIGN KEY (guild)
                        REFERENCES guilds(guild)
                        ON DELETE CASCADE,
             CONSTRAINT fk_roles
                        FOREIGN KEY (guild, role)
                        REFERENCES roles(guild, role)
                        ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS reaction_roles (
             guild           BIGINT,
             channel         BIGINT,
             message         BIGINT,
             emoji           INT,
             role            BIGINT,
             slots           INT,
             PRIMARY KEY (guild, channel, message, emoji, role),
             CONSTRAINT fk_guilds
                        FOREIGN KEY (guild)
                        REFERENCES guilds(guild)
                        ON DELETE CASCADE,
             CONSTRAINT fk_channels
                        FOREIGN KEY (guild, channel)
                        REFERENCES channels(guild, channel)
                        ON DELETE CASCADE,
             CONSTRAINT fk_messages
                        FOREIGN KEY (guild, channel, message)
                        REFERENCES messages(guild, channel, message)
                        ON DELETE CASCADE,
             CONSTRAINT fk_emojis
                        FOREIGN KEY (emoji)
                        REFERENCES emojis(id)
                        ON DELETE CASCADE,
             CONSTRAINT fk_roles
                        FOREIGN KEY (guild, role)
                        REFERENCES roles(guild, role)
                        ON DELETE CASCADE,
             CONSTRAINT unsigned_slots
                         CHECK (slots >= 0)
);

CREATE TABLE IF NOT EXISTS reminders (
                guild           BIGINT,
                channel         BIGINT,
                message         BIGINT,
                "user"        BIGINT,
                time            TIMESTAMP WITH TIME ZONE,
                content         TEXT NOT NULL,
                CONSTRAINT fk_guilds
                           FOREIGN KEY (guild)
                           REFERENCES guilds(guild)
                           ON DELETE CASCADE,
                CONSTRAINT fk_channels
                           FOREIGN KEY (guild, channel)
                           REFERENCES channels(guild, channel)
                           ON DELETE CASCADE,
                CONSTRAINT fk_messages
                           FOREIGN KEY (guild, channel, message)
                           REFERENCES messages(guild, channel, message)
                           ON DELETE CASCADE,
                CONSTRAINT fk_users
                           FOREIGN KEY (guild, "user")
                           REFERENCES users(guild, "user")
                           ON DELETE CASCADE,
                PRIMARY KEY (guild, channel, "user", time)
);

CREATE TABLE IF NOT EXISTS owned_guilds (
             guild           BIGINT PRIMARY KEY,
             CONSTRAINT fk_guilds
                        FOREIGN KEY (guild)
                        REFERENCES guilds(guild)
                        ON DELETE CASCADE
);
