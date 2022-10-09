-- Add migration script here

ALTER TABLE users ADD COLUMN id BIGSERIAL;
ALTER TABLE score_reactions DROP CONSTRAINT fk_users;
ALTER TABLE reminders DROP CONSTRAINT fk_users;
ALTER TABLE users DROP CONSTRAINT users_pkey;
ALTER TABLE users ADD PRIMARY KEY(id);
ALTER TABLE users ADD CONSTRAINT unique_guild_user UNIQUE(guild,"user");
ALTER TABLE users ALTER COLUMN guild SET NOT NULL;
ALTER TABLE users ALTER COLUMN "user" SET NOT NULL;

ALTER TABLE channels ADD COLUMN id BIGSERIAL;
ALTER TABLE messages DROP CONSTRAINT fk_channels;
ALTER TABLE score_drops DROP CONSTRAINT fk_channels;
ALTER TABLE reaction_roles DROP CONSTRAINT fk_channels;
ALTER TABLE reminders DROP CONSTRAINT fk_channels;
ALTER TABLE channels DROP CONSTRAINT channels_pkey;
ALTER TABLE channels ADD PRIMARY KEY(id);
ALTER TABLE channels ADD CONSTRAINT unique_guild_channel UNIQUE(guild,channel);
ALTER TABLE channels ALTER COLUMN guild SET NOT NULL;
ALTER TABLE channels ALTER COLUMN channel SET NOT NULL;

ALTER TABLE roles ADD COLUMN id BIGSERIAL;
ALTER TABLE score_cooldowns DROP CONSTRAINT fk_roles;
ALTER TABLE score_roles DROP CONSTRAINT fk_roles;
ALTER TABLE reaction_roles DROP CONSTRAINT fk_roles;
ALTER TABLE roles DROP CONSTRAINT roles_pkey;
ALTER TABLE roles ADD PRIMARY KEY(id);
ALTER TABLE roles ADD CONSTRAINT unique_guild_role UNIQUE(guild,role);
ALTER TABLE roles ALTER COLUMN guild SET NOT NULL;
ALTER TABLE roles ALTER COLUMN role SET NOT NULL;

ALTER TABLE reaction_roles DROP CONSTRAINT fk_messages;
ALTER TABLE reminders DROP CONSTRAINT fk_messages;
UPDATE messages
       SET channel=channels.id FROM channels
       WHERE channels.channel=messages.channel AND channels.guild=messages.guild;
ALTER TABLE messages ADD COLUMN id BIGSERIAL;
ALTER TABLE messages DROP CONSTRAINT messages_pkey;
ALTER TABLE messages ADD PRIMARY KEY(id);
ALTER TABLE messages ADD CONSTRAINT unique_fields UNIQUE(channel, message);
ALTER TABLE messages ALTER channel SET NOT NULL;
ALTER TABLE messages ALTER message SET NOT NULL;
ALTER TABLE messages DROP CONSTRAINT fk_guilds;
ALTER TABLE messages DROP COLUMN guild;
ALTER TABLE messages
            ADD CONSTRAINT fk_channels
                FOREIGN KEY (channel)
                REFERENCES channels(id)
                ON DELETE CASCADE;

ALTER TABLE score_emojis DROP CONSTRAINT fk_emojis;
ALTER TABLE reaction_roles DROP CONSTRAINT fk_emojis;
ALTER TABLE emojis ALTER COLUMN id Type BIGINT;

ALTER TABLE modules ADD COLUMN owner BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE modules ADD COLUMN utility BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE modules ADD COLUMN score BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE modules ADD COLUMN reaction_roles BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE modules ADD COLUMN "analyze" BOOLEAN NOT NULL DEFAULT false;
UPDATE modules
       SET (owner,utility,score,reaction_roles,"analyze")=(
            status&B'00000001'=B'00000001',
            status&B'00000010'=B'00000010',
            status&B'00000100'=B'00000100',
            status&B'00001000'=B'00001000',
            status&B'00010000'=B'00010000'
);
ALTER TABLE modules DROP COLUMN status;

ALTER TABLE publishing ALTER COLUMN guild SET NOT NULL;

UPDATE score_cooldowns
       SET role=roles.id FROM roles
       WHERE roles.role=score_cooldowns.role AND roles.guild=score_cooldowns.guild;
ALTER TABLE score_cooldowns DROP CONSTRAINT score_cooldowns_pkey;
ALTER TABLE score_cooldowns ADD PRIMARY KEY(role);
ALTER TABLE score_cooldowns DROP CONSTRAINT fk_guilds;
ALTER TABLE score_cooldowns DROP COLUMN guild;
ALTER TABLE score_cooldowns
            ADD CONSTRAINT fk_roles
                FOREIGN KEY (role)
                REFERENCES roles(id)
                ON DELETE CASCADE;

UPDATE score_drops
       SET channel=channels.id FROM channels
       WHERE channels.channel=score_drops.channel AND channels.guild=score_drops.guild;
ALTER TABLE score_drops DROP CONSTRAINT score_drops_pkey;
ALTER TABLE score_drops ADD PRIMARY KEY(channel);
ALTER TABLE score_drops DROP CONSTRAINT fk_guilds;
ALTER TABLE score_drops DROP COLUMN guild;
ALTER TABLE score_drops
            ADD CONSTRAINT fk_channels
                FOREIGN KEY (channel)
                REFERENCES channels(id)
                ON DELETE CASCADE;

ALTER TABLE score_emojis ADD COLUMN id BIGSERIAL;
ALTER TABLE score_reactions DROP CONSTRAINT fk_score_emojis;
ALTER TABLE score_emojis DROP CONSTRAINT score_emojis_pkey;
ALTER TABLE score_emojis ALTER COLUMN emoji Type BIGINT;
ALTER TABLE score_emojis ALTER COLUMN guild SET NOT NULL;
ALTER TABLE score_emojis ALTER COLUMN emoji SET NOT NULL;
ALTER TABLE score_emojis ADD PRIMARY KEY(id);
ALTER TABLE score_emojis
            ADD CONSTRAINT fk_emojis
                FOREIGN KEY (emoji)
                REFERENCES emojis(id)
                ON DELETE CASCADE;
ALTER TABLE score_emojis ADD CONSTRAINT unique_guild_emoji UNIQUE(guild,emoji);



UPDATE score_reactions
       SET user_to=users.id FROM users
       WHERE users.user=score_reactions.user_to AND users.guild=score_reactions.guild;
ALTER TABLE score_reactions ALTER COLUMN emoji TYPE BIGINT;
UPDATE score_reactions
       SET emoji=score_emojis.id FROM score_emojis
       WHERE score_emojis.emoji=score_reactions.emoji AND score_emojis.guild=score_reactions.guild;
ALTER TABLE score_reactions ADD COLUMN id BIGSERIAL;
ALTER TABLE score_reactions DROP CONSTRAINT score_reactions_pkey;
ALTER TABLE score_reactions ADD PRIMARY KEY(id);
ALTER TABLE score_reactions ALTER COLUMN user_from SET NOT NULL;
ALTER TABLE score_reactions ALTER COLUMN user_to SET NOT NULL;
ALTER TABLE score_reactions ALTER COLUMN message SET NOT NULL;
ALTER TABLE score_reactions ALTER COLUMN channel SET NOT NULL;
ALTER TABLE score_reactions ALTER COLUMN emoji SET NOT NULL;
ALTER TABLE score_reactions ADD CONSTRAINT unique_score_reaction UNIQUE(user_from,message,channel,emoji);
ALTER TABLE score_reactions DROP CONSTRAINT fk_guilds;
ALTER TABLE score_reactions DROP COLUMN guild;
ALTER TABLE score_reactions
            ADD CONSTRAINT fk_score_emojis
                FOREIGN KEY (emoji)
                REFERENCES score_emojis(id)
                ON DELETE CASCADE;
ALTER TABLE score_reactions
            ADD CONSTRAINT fk_users
                FOREIGN KEY (user_to)
                REFERENCES users(id)
                ON DELETE CASCADE;

UPDATE score_roles
       SET role=roles.id FROM roles
       WHERE roles.role=score_roles.role AND roles.guild=score_roles.guild;
ALTER TABLE score_roles ADD COLUMN id BIGSERIAL;
ALTER TABLE score_roles DROP CONSTRAINT score_roles_pkey;
ALTER TABLE score_roles DROP CONSTRAINT fk_guilds;
ALTER TABLE score_roles DROP COLUMN guild;
ALTER TABLE score_roles ADD PRIMARY KEY(id);
ALTER TABLE score_roles ALTER COLUMN role SET NOT NULL;
ALTER TABLE score_roles ALTER COLUMN score SET NOT NULL;
ALTER TABLE score_roles ADD CONSTRAINT unique_role_score UNIQUE(role,score);
ALTER TABLE score_roles
            ADD CONSTRAINT fk_roles
                FOREIGN KEY (role)
                REFERENCES roles(id)
                ON DELETE CASCADE;

UPDATE reaction_roles
       SET message=messages.id FROM messages INNER JOIN channels ON messages.channel=channels.id
       WHERE messages.message=reaction_roles.message AND channels.channel=reaction_roles.channel AND channels.guild=reaction_roles.guild;
UPDATE reaction_roles
       SET role=roles.id FROM roles
       WHERE roles.role=reaction_roles.role AND roles.guild=reaction_roles.guild;
ALTER TABLE reaction_roles ADD COLUMN id BIGSERIAL;
ALTER TABLE reaction_roles DROP CONSTRAINT reaction_roles_pkey;
ALTER TABLE reaction_roles DROP CONSTRAINT fk_guilds;
ALTER TABLE reaction_roles DROP COLUMN guild;
ALTER TABLE reaction_roles DROP COLUMN channel;
ALTER TABLE reaction_roles ALTER COLUMN emoji Type BIGINT;
ALTER TABLE reaction_roles ADD PRIMARY KEY(id);
ALTER TABLE reaction_roles ALTER COLUMN message SET NOT NULL;
ALTER TABLE reaction_roles ALTER COLUMN emoji SET NOT NULL;
ALTER TABLE reaction_roles ALTER COLUMN role SET NOT NULL;
ALTER TABLE reaction_roles ADD CONSTRAINT unique_reaction_role UNIQUE(message,emoji,role);
ALTER TABLE reaction_roles
            ADD CONSTRAINT fk_emojis
                FOREIGN KEY (emoji)
                REFERENCES emojis(id)
                ON DELETE CASCADE;
ALTER TABLE reaction_roles
            ADD CONSTRAINT fk_roles
                FOREIGN KEY (role)
                REFERENCES roles(id)
                ON DELETE CASCADE;
ALTER TABLE reaction_roles
            ADD CONSTRAINT fk_messages
                FOREIGN KEY (message)
                REFERENCES messages(id)
                ON DELETE CASCADE;

UPDATE reminders
       SET message=messages.id FROM messages INNER JOIN channels ON messages.channel=channels.id
       WHERE messages.message=reminders.message AND channels.channel=reminders.channel AND channels.guild=reminders.guild;
UPDATE reminders
       SET "user"=users.id FROM users
       WHERE users."user"=reminders."user" AND users.guild=reminders.guild;
ALTER TABLE reminders ADD COLUMN id BIGSERIAL;
ALTER TABLE reminders DROP CONSTRAINT fk_guilds;
ALTER TABLE reminders DROP CONSTRAINT reminders_pkey;
ALTER TABLE reminders DROP COLUMN guild;
ALTER TABLE reminders DROP COLUMN channel;
ALTER TABLE reminders ADD PRIMARY KEY(id);
ALTER TABLE reminders ADD CONSTRAINT unique_reminders UNIQUE(message,"user");
ALTER TABLE reminders ALTER COLUMN message SET NOT NULL;
ALTER TABLE reminders ALTER COLUMN "user" SET NOT NULL;
ALTER TABLE reminders ALTER COLUMN time SET NOT NULL;
ALTER TABLE reminders
            ADD CONSTRAINT fk_messages
                FOREIGN KEY (message)
                REFERENCES messages(id)
                ON DELETE CASCADE;
ALTER TABLE reminders
            ADD CONSTRAINT fk_users
                FOREIGN KEY ("user")
                REFERENCES users(id)
                ON DELETE CASCADE;
