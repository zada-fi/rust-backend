-- Your SQL goes here
-- store amm events
CREATE TABLE events (
     id serial NOT NULL,
     tx_hash text NOT NULL,
     event_type integer NOT NULL,-- add_liq/swap/rm_liq
     pair_address text NOT NULL,
     from_account text NOT NULL,
     to_account text,
     amount_x numeric , -- while swap if x->y :x>0&&y==0 else y>0 && x==0;while add_liq x,y > 0
     amount_y numeric,
     PRIMARY KEY (id)
);

-- store amm pool info
CREATE TABLE pool_info (
    id serial NOT NULL,
    pair_address text NOT NULL,
    token_x_symbol text NOT NULL,
    token_y_symbol text NOT NULL,
    token_x_address text NOT NULL,
    token_y_address text NOT NULL,
    token_x_reserves numeric NOT NULL,
    token_y_reserves numeric NOT NULL,
    total_swap_count integer NOT NULL,
    total_add_liq_count integer NOT NULL,
    total_rm_liq_count integer NOT NULL,
    PRIMARY KEY (id)
);

-- store tokens info
CREATE TABLE tokens (
   address text NOT NULL,
   symbol text NOT NULL,
   decimals integer NOT NULL,
   PRIMARY KEY (address)
);
-- last sync block number
CREATE TABLE last_sync_block (
   block_number bigint NOT NULL,
   PRIMARY KEY (block_number)
);