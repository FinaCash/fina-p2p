use cosmwasm_std::{Deps, StdResult};

use crate::{msg::QueryAnswer, state::{ACTIVE_DEALS, ACTIVE_POSTS, CONFIG, MODERATORS, PAST_DEALS, TOKEN_REVENUE}};



pub fn config(deps: Deps) -> StdResult<QueryAnswer> {
    Ok(QueryAnswer::Config {
        config: CONFIG.load(deps.storage)?,
    })
}

pub fn past_deals(deps: Deps) -> StdResult<QueryAnswer> {
    Ok(QueryAnswer::PastDeals {
        past_deals: PAST_DEALS.load(deps.storage)?,
    })
}

pub fn active_deals(deps: Deps) -> StdResult<QueryAnswer> {
    Ok(QueryAnswer::ActiveDeals {
        deals: ACTIVE_DEALS.load(deps.storage)?,
    })
}

pub fn active_posts(deps: Deps) -> StdResult<QueryAnswer> {
    Ok(QueryAnswer::ActivePosts {
        posts: ACTIVE_POSTS.load(deps.storage)?,
    })
}

pub fn my_deals(deps: Deps) -> StdResult<QueryAnswer> {
    Ok(QueryAnswer::MyDeals {
        deals: ACTIVE_DEALS.load(deps.storage)?,
    })
}

pub fn revenue(deps: Deps) -> StdResult<QueryAnswer> {
    Ok(QueryAnswer::Revenue {
        revenue: TOKEN_REVENUE.load(deps.storage)?,
    })
}

pub fn moderators(deps: Deps) -> StdResult<QueryAnswer> {
    Ok(QueryAnswer::Moderators {
        mods: MODERATORS.load(deps.storage)?,
    })
}