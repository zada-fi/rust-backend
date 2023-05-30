-- Your SQL goes here
-- store amm events
CREATE TABLE events (
     id serial NOT NULL,
     tx_hash text NOT NULL,
     event_type integer NOT NULL,-- add_liq/swap/rm_liq
     pair_address text NOT NULL,
     from_account text,
     to_account text,
     amount_x numeric , -- while swap if x->y :x>0&&y==0 else y>0 && x==0;while add_liq x,y > 0
     amount_y numeric,
     event_time timestamp with time zone,
     is_swap_x_y bool,
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
   coingecko_id text,
   usd_price numeric,
   PRIMARY KEY (address)
);

-- store priceCumulativeLast
CREATE TABLE price_cumulative_last (
    id                     serial  NOT NULL,
    pair_address           text    NOT NULL,
    price0_cumulative_last numeric NOT NULL,
    price1_cumulative_last numeric NOT NULL,
    block_timestamp_last   integer NOT NULL
);
-- last sync block number
CREATE TABLE last_sync_block (
   block_number bigint NOT NULL,
   PRIMARY KEY (block_number)
);

-- store tvl and volume every day
CREATE TABLE event_stats (
--     id serial NOT NULL,
    pair_address text NOT NULL,
    stat_date date NOT NULL,
    x_reserves numeric NOT NULL,
    y_reserves numeric NOT NULL,
    x_volume numeric NOT NULL,
    y_volume numeric NOT NULL,
    usd_tvl numeric NOT NULL,
    usd_volume numeric NOT NULL,
    PRIMARY KEY (pair_address,stat_date)
)