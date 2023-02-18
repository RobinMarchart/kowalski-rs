-- Add migration script here
CREATE VIEW guilds_v AS SELECT * FROM guilds;

CREATE VIEW users_v AS
       SELECT guild,"user",id AS _user_id FROM users;

CREATE VIEW channels_v AS
       SELECT guild,channel,id AS _channel_id FROM channels;

CREATE VIEW roles_v AS
       SELECT guild, role, id AS _role_id FROM roles;

CREATE VIEW messages_v AS
       SELECT c.* , m.message, m.id AS _message_id
       FROM channels_v AS c INNER JOIN messages AS m ON c._channel_id = m.channel;

CREATE VIEW emojis_v AS
       SELECT unicode,guild,guild_emoji,NOT guild IS NULL AS is_guild_emoji, id AS _emoji_id FROM emojis;

CREATE VIEW modules_v AS SELECT * FROM modules;

CREATE VIEW publishing_v AS SELECT * FROM publishing;

CREATE VIEW score_auto_delete_v AS SELECT * FROM score_auto_delete;

CREATE VIEW score_auto_pin_v AS SELECT * FROM score_auto_pin;

CREATE VIEW score_cooldowns_v AS
       SELECT r.*, s.cooldown
        FROM score_cooldowns AS s INNER JOIN roles_v AS r ON s.role=r._role_id;

CREATE VIEW score_drops_v AS
       SELECT c.*
       FROM score_drops AS s INNER JOIN channels_v AS c ON s.channel=c._channel_id;

CREATE VIEW score_emojis_v AS
       SELECT s.guild, s.upvote, s.id AS _score_emoji_id,
              e.unicode,e.guild AS emoji_source_guild,e.guild_emoji,e.is_guild_emoji, e._emoji_id,
              (s.guild != e.guild) AS is_extern_emoji
       FROM score_emojis AS s INNER JOIN emojis_v AS e ON s.emoji=e._emoji_id;

CREATE VIEW score_reactions_v AS
       SELECT "to".guild,
              s.user_from,"from"._user_id AS _user_from_id,
              "to"."user" AS user_to, "to"._user_id AS _user_to_id,
              s.message,m.id AS _message_id,
              s.channel,c._channel_id,
              e.upvote,e._score_emoji_id,e.unicode,e.emoji_source_guild,e.guild_emoji,e.is_guild_emoji,e._emoji_id,e.is_extern_emoji,
              s.native
        FROM score_reactions AS s
             INNER JOIN users_v AS "to" ON s.user_to="to"._user_id
             LEFT OUTER JOIN users_v AS "from" ON s.user_from="from".user AND "to".guild="from".guild
             LEFT OUTER JOIN channels_v AS c ON s.channel=c.channel AND "to".guild=c.guild
             LEFT OUTER JOIN messages AS m ON c._channel_id=m.channel AND s.message=m.message
             INNER JOIN score_emojis_v AS e ON s.emoji=e._score_emoji_id;

CREATE VIEW score_roles_v AS
       SELECT r.*,s.score FROM score_roles AS s INNER JOIN roles_v AS r ON s.role=r._role_id;

CREATE VIEW reaction_roles_v AS
       SELECT m.*,
              e.unicode,e.guild AS emoji_source_guild,e.guild_emoji,e.is_guild_emoji,e._emoji_id,
              r.role,r._role_id,
              rr.slots, rr.id AS _reaction_roles_id,
              (m.guild != e.guild) AS is_extern_emoji
       FROM reaction_roles AS rr
            INNER JOIN messages_v AS m ON rr.message=m._message_id
            INNER JOIN emojis_v AS e ON rr.emoji=e._emoji_id
            INNER JOIN roles_v AS r ON rr.role=r._role_id;

CREATE VIEW reminders_v AS
       SELECT m.*,u."user",u._user_id,r.time,r.content,r.id AS _reminder_id
       FROM reminders AS r
       INNER JOIN messages_v AS m ON r.message=m._message_id
       INNER JOIN users_v AS u ON r.user=u._user_id;

CREATE VIEW owned_guilds_v AS SELECT * FROM owned_guilds;
