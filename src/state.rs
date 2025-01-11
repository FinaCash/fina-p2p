use cosmwasm_schema::cw_serde;

use cosmwasm_std::{Addr, Uint128};

use secret_toolkit::storage::{Item, Keymap};
use secret_toolkit::serialization::Json;

use crate::asset::{Contract, DealState, PostState};

// single value store
pub const CONFIG: Item<Config, Json> = Item::new(b"config");
pub const MAX_DEAL_ID: Item<Uint128> = Item::new(b"max_deal_id");
pub const MAX_POST_ID: Item<Uint128> = Item::new(b"max_post_id");
pub const TOKEN_REVENUE: Item<Uint128> = Item::new(b"token_revenue");

// vector value store (a list of items)
pub const PAST_DEALS: Item<Vec<Deal>, Json> = Item::new(b"deals");
pub const ACTIVE_POSTS: Item<Vec<Post>, Json> = Item::new(b"active_posts");
pub const ACTIVE_DEALS: Item<Vec<Deal>, Json> = Item::new(b"active_deals");
pub const MODERATORS: Item<Vec<Addr>, Json> = Item::new(b"moderators");

// map value store (per user usually)
pub const USER_PAYMENT_INFO: Keymap<Addr, PaymentInfo, Json> = Keymap::new(b"user_payment_info");
pub const USER_COOL_DOWN: Keymap<Addr, Uint128, Json> = Keymap::new(b"user_cool_down");

#[cw_serde]
pub struct Config {
    pub admins: Vec<Addr>,
    pub deal_commission: Uint128,  // in bps, 1 bps = 0.01% commision
    pub deal_token_a: Contract,
    pub deal_token_b: Contract,
    pub deal_token_c: Contract,
    pub query_auth: Contract,
    pub governance: Option<Contract>,
}

#[cw_serde]
pub struct Post {
    pub post_id: Uint128,
    pub is_dealer_buy: bool, // is dealer buying crypto
    pub deal_token: Contract,
    pub amount: Uint128, // number of crypto, also this is the remaining of the post amount
    pub min_amount: Uint128, // min amount allowed to init the deal
    pub settle_currency: String,
    pub settle_price: Uint128,
    pub dealer_deposit: bool,
    pub dealer: Addr,
    pub state: PostState,
    pub expiry: Uint128,
}

#[cw_serde]
pub struct Deal {
    pub deal_id: Uint128,
    pub post_id: Uint128,  // corresponding post that associated with the deal
    pub is_dealer_buy: bool, // is dealer buying crypto
    pub deal_token: Contract,
    pub amount: Uint128, // number of crypto
    pub settle_currency: String,
    pub settle_price: Uint128,
    pub dealer_deposit: bool,
    pub customer_deposit: bool,
    pub dealer: Addr,
    pub customer: Addr,
    pub state: DealState,
    pub resolver: Option<Addr>,
    pub expiry: Option<Uint128>,
}

#[cw_serde]
pub struct PaymentInfo {
    pub method: String,
    pub detail: String,
}