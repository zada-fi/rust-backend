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
     is_swap_x2y bool,
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
CREATE TABLE tvl_stats (
--     id serial NOT NULL,
    pair_address text NOT NULL,
    stat_date date NOT NULL,
    x_reserves numeric NOT NULL,
    y_reserves numeric NOT NULL,
    usd_tvl numeric NOT NULL,
    PRIMARY KEY (pair_address,stat_date)
)
CREATE TABLE volume_stats (
--     id serial NOT NULL,
   pair_address text NOT NULL,
   stat_date date NOT NULL,
   x_volume numeric NOT NULL,
   y_volume numeric NOT NULL,
   usd_volume numeric NOT NULL,
   PRIMARY KEY (pair_address,stat_date)
)
CREATE TABLE history_stats (
  stat_date date NOT NULL,
  usd_tvl numeric NOT NULL,
  usd_volume numeric NOT NULL,
  PRIMARY KEY (stat_date)
)
-- store launchpad projects
CREATE TABLE projects (
    project_name text NOT NULL,
    project_address text NOT NULL,
    project_description text,
    project_links json,
    project_owner text NOT NULL,
    token_address text NOT NULL,
    token_price numeric NOT NULL,
    start_time timestamp NOT NULL,
    end_time timestamp NOT NULL,
    raise_limit numeric NOT NULL,
    raised numeric NOT NULL DEFAULT 0,
    paused bool NOT NULL DEFAULT false,
    purchased_min_limit integer NOT NULL,
    purchased_max_limit integer NOT NULL,
    created_time timestamp NOT NULL default now(),
    last_updated_time timestamp,
    PRIMARY KEY (project_name)
)

-- -- store launchpad whitelist
-- CREATE TABLE project_white_lists (
--     id serial NOT NULL,
--     project_address text NOT NULL,
--     user_address text NOT NULL,
-- )
-- store user invest evnets
CREATE TABLE project_addresses (
    project_name text NOT NULL,
    project_address text NOT NULL,
    PRIMARY KEY (project_name)
)
-- store user invest evnets
CREATE TABLE user_invest_events (
    id serial NOT NULL,
    tx_hash text NOT NULL,
    project_address text NOT NULL,
    user text NOT NULL,
    amount numeric NOT NULL,
    invest_time timestamp with time zone,
)