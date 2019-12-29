/*
The giveaway in the chat room.

description  : Description for the giveaway
participants : List of participants in JSON format
finished     : Defines the state of the whole giveaway
created_at   : The datetime when the giveaway was created by someone
*/
CREATE TABLE giveaway (
  id SERIAL PRIMARY KEY,
  description TEXT NOT NULL DEFAULT '',
  participants JSONB NOT NULL DEFAULT '[]'::jsonb,
  finished BOOLEAN NOT NULL DEFAULT FALSE,
  created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
);

/*
Represent an object for the giveaway.

giveaway_id  : Foreign key to the giveaway
value        : The item description / code / key / etc.
object_key   : Enum -> Key | Other
object_state : Enum -> Activated | Pending | Unused
*/
CREATE TABLE giveaway_object (
  id SERIAL PRIMARY KEY,
  giveaway_id INTEGER NOT NULL REFERENCES giveaway(id),
  value TEXT NOT NULL,
  object_type VARCHAR(255) NOT NULL,
  object_state VARCHAR(255) NOT NULL
);