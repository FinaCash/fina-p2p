use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::asset::RawContract;
use crate::state::{Config, Deal, PaymentInfo, Post};


#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub admins: Vec<String>,
    pub deal_commission: Uint128,  // in number of bps, 1 = 0.01% of the amount of the deal to be commission
    pub deal_token: RawContract,
    pub query_auth: RawContract,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    UpdateConfig {
        admins: Option<Vec<String>>,
        deal_commission: Option<Uint128>,
        query_auth: Option<RawContract>,
        governance: Option<RawContract>,
    },
    UpdateDealToken {
        deal_token: RawContract,
    },
    AddModerator {
        mod_addr: String,
    },
    RemoveModerator {
        mod_addr: String,
    },
    RegisterPaymentInfo {
        method: String,
        detail: String
    },
    Receive { 
        sender: Addr,
        from: Addr,
        amount: Uint128,
        memo: Option<Binary>,
        msg: Option<Binary>,
    }, // must be in secret or fina
    AddPost {
        is_dealer_buy: bool,  // otherwise dealer is selling crypto
        amount: Uint128,  // amount of snip 20 token, e.g. 1_000_000 = 1
        min_amount: Uint128, // min amount required by dealer to open a deal
        settle_currency: String,
        settle_price: Uint128,  // price, also expressed as 1_000_000 = 1, so 50_000 = $0.05
    },
    CancelPost { post_id: Uint128 },
    EnterDeal { 
        post_id: Uint128,
        amount: Uint128,  // amount of snip 20 token that customer wanna trade from the post
    },
    ConfirmBankTransfer { deal_id: Uint128 },
    DisputeDeal { deal_id: Uint128 },
    ResolveDeal { deal_id: Uint128 },
    CancelDeal { deal_id: Uint128 },
    AdminDeleteDeal { deal_id: Uint128 },
    // EmergencyWithdraw { deal_id: Uint128 },  // only for testing
    GetCommission {},
}

#[cw_serde]
pub enum ExecuteAnswer {
    UpdateConfig {
        status: ResponseStatus,
    },
    UpdateDealToken {
        status: ResponseStatus,
    },
    AddModerator {
        status: ResponseStatus,
    },
    RemoveModerator {
        status: ResponseStatus,
    },
    RegisterPaymentInfo {
        status: ResponseStatus
    },
    CustomerDeposit {
        status: ResponseStatus
    },
    AddPost {
        status: ResponseStatus,
        post_id: Uint128
    },
    CancelPost {
        status: ResponseStatus,
        post_id: Uint128
    },
    DealStageProcess {
        status: ResponseStatus,
        deal_id: Uint128
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // GetCount returns the current count as a json-encoded number
    Config {},
    PastDeals {},
    ActiveDeals {},
    ActivePosts {},
    MyPosts { key: String, address: String },
    MyDeals { key: String, address: String },
    MyPaymentInfo { key: String, address: String },
    DealDetail { key: String, address: String, deal_id: Uint128 },
    Revenue {},
    Moderators {},
}

#[cw_serde]
pub enum AuthQueryMsg {
    ValidateViewingKey { user: Addr, key: String },
}

#[cw_serde]
pub enum AuthQueryAnswer {
    ValidateViewingKey { is_valid: bool },
}

#[cw_serde]
pub enum QueryAnswer {
    Config {
        config: Config,
    },
    PastDeals {
        past_deals: Vec<Deal>,
    },
    ActiveDeals {
        deals: Vec<Deal>,
    },
    ActivePosts {
        posts: Vec<Post>,
    },
    MyDeals {
        deals: Vec<Deal>,
    },
    MyPaymentInfo {
        payment_info: PaymentInfo,
    },
    DealDetail {
        deal: Deal,
        payment_info: PaymentInfo
    },
    Revenue {
        revenue: Uint128,
    },
    Moderators {
        mods: Vec<Addr>,
    }
}


#[cw_serde]
pub enum ResponseStatus {
    Success,
    Failure,
}
