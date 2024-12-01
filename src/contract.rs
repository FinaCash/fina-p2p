use cosmwasm_std::{
    entry_point, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, QueryRequest, Response, StdError, StdResult, Uint128, WasmQuery
};
use secret_toolkit::snip20::register_receive_msg;
use secret_toolkit::utils::pad_handle_result;

use crate::asset::{Contract, ContractError};
use crate::msg::{AuthQueryAnswer, AuthQueryMsg, ExecuteMsg, InstantiateMsg, QueryAnswer, QueryMsg};
use crate::state::{Config, Deal, PaymentInfo, Post, ACTIVE_DEALS, ACTIVE_POSTS, CONFIG, MAX_DEAL_ID, MAX_POST_ID, MODERATORS, PAST_DEALS, TOKEN_REVENUE, USER_PAYMENT_INFO};
use crate::{execute, query};

pub const RESPONSE_BLOCK_SIZE: usize = 256;


// Because of deal multiplier, deal amount will only support 2 decimal
// But I won't enforce it here as the contract will simply throw panic error
#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let admins_addr: Vec<Addr> = msg.admins.iter()
        .map(|s| deps.api.addr_validate(s).unwrap())
        .collect();
    let moderators = admins_addr.clone();
    let deal_token = msg.deal_token.into_valid(deps.api)?;

    CONFIG.save(
        deps.storage,
        &Config {
            admins: admins_addr,
            deal_commission: msg.deal_commission.clone(),
            deal_token: deal_token.clone(),
            query_auth: msg.query_auth.into_valid(deps.api)?,
            governance: None,
        }
    )?;

    MAX_DEAL_ID.save(deps.storage, &Uint128::zero())?;
    MAX_POST_ID.save(deps.storage, &Uint128::zero())?;
    TOKEN_REVENUE.save(deps.storage, &Uint128::zero())?;
    PAST_DEALS.save(deps.storage, &vec![])?;
    ACTIVE_POSTS.save(deps.storage, &vec![])?;
    ACTIVE_DEALS.save(deps.storage, &vec![])?;
    // initially, moderators is admins
    MODERATORS.save(deps.storage, &moderators)?;

    let response = Response::new()
        .add_messages(vec![
            register_receive_msg(
                env.contract.code_hash.clone(),
                None,
                RESPONSE_BLOCK_SIZE,
                deal_token.code_hash.clone(),
                deal_token.address.into_string().clone())?
        ])
        .add_attribute("status", "success");

    Ok(response)
}

#[entry_point]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg
) -> Result<Response, ContractError> {
    pad_handle_result(
        match msg {
        ExecuteMsg::UpdateConfig {
            admins,
            deal_commission,
            query_auth,
            governance
        } => execute::update_config(
            deps,
            env,
            info,
            admins,
            deal_commission,
            query_auth,
            governance
        ),
        ExecuteMsg::UpdateDealToken { deal_token } => {
            execute::update_deal_token(deps, env, info, deal_token)
        },
        ExecuteMsg::AddModerator { mod_addr } => {
            execute::add_moderator(deps, env, info, mod_addr)
        },
        ExecuteMsg::RemoveModerator { mod_addr } => {
            execute::remove_moderator(deps, env, info, mod_addr)
        },
        ExecuteMsg::RegisterPaymentInfo { method, detail } => {
            execute::register_user_payment_info(deps, env, info, method, detail)
        },
        ExecuteMsg::Receive {
            sender,
            from,
            amount,
            msg,
            ..
        } => execute::receive(deps, env, info, sender, from, amount, msg),
        ExecuteMsg::AddPost {
            is_dealer_buy,
            amount,
            min_amount,
            settle_currency,
            settle_price
        } => execute::add_post(deps, env, info, is_dealer_buy, amount, min_amount, settle_currency, settle_price),
        ExecuteMsg::CancelPost { post_id } => execute::cancel_post(deps, env, info, post_id),
        ExecuteMsg::EnterDeal { 
            post_id,
            amount
        } => execute::enter_deal(deps, env, info, post_id, amount),
        ExecuteMsg::ConfirmBankTransfer { 
            deal_id 
        } => execute::confirm_bank_transfer(deps, env, info, deal_id),
        ExecuteMsg::DisputeDeal { deal_id } => execute::dispute_deal(deps, env, info, deal_id),
        ExecuteMsg::ResolveDeal { deal_id } => execute::resolve_deal(deps, env, info, deal_id),
        ExecuteMsg::CancelDeal { deal_id } => execute::cancel_deal(deps, env, info, deal_id),
        ExecuteMsg::EmergencyWithdraw { deal_id } => execute::emergency_withdraw(deps, env, info, deal_id),
        ExecuteMsg::AdminDeleteDeal { deal_id } => execute::admin_delete_deal(deps, env, info, deal_id),
        ExecuteMsg::GetCommission {} => execute::get_commission(deps, env, info)
    },
    RESPONSE_BLOCK_SIZE)
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config{} => to_binary(&query::config(deps)?),
        QueryMsg::PastDeals {} => to_binary(&query::past_deals(deps)?),
        QueryMsg::ActiveDeals {} => to_binary(&query::active_deals(deps)?),
        QueryMsg::ActivePosts {} => to_binary(&query::active_posts(deps)?),
        QueryMsg::Revenue {} => to_binary(&query::revenue(deps)?),
        QueryMsg::Moderators {} => to_binary(&query::moderators(deps)?),
        QueryMsg::MyPosts {
            key,
            address
        } => {
            let config = CONFIG.load(deps.storage)?;
            let is_valid = authenticate(deps, key, address.clone(), config.query_auth)?;

            if !is_valid {
                return Err(StdError::generic_err("Viewing key not match"));
            }

            let user = deps.api.addr_validate(&address)?;

            let active_posts = ACTIVE_POSTS.load(deps.storage)?;

            let mut list_posts: Vec<Post> = Vec::new();
            
            for post in active_posts {
                if user == post.dealer.clone() {
                    list_posts.push(post.clone());
                }
            }

            to_binary(&list_posts)
        },
        QueryMsg::MyDeals {
            key,
            address
        } => {
            let config = CONFIG.load(deps.storage)?;
            let is_valid = authenticate(deps, key, address.clone(), config.query_auth)?;

            if !is_valid {
                return Err(StdError::generic_err("Viewing key not match"));
            }

            let user = deps.api.addr_validate(&address)?;

            let active_deals = ACTIVE_DEALS.load(deps.storage)?;

            let mut list_deals: Vec<Deal> = Vec::new();
            
            for deal in active_deals {
                let customer = deal.customer.clone();
                if user == customer || user == deal.dealer.clone() {
                    list_deals.push(deal.clone());
                }
            }

            to_binary(&list_deals)
        },
        QueryMsg::MyPaymentInfo {
            key,
            address
        } => {
            let config = CONFIG.load(deps.storage)?;
            let is_valid = authenticate(deps, key, address.clone(), config.query_auth)?;

            if !is_valid {
                return Err(StdError::generic_err("Viewing key not match"));
            }

            let user = deps.api.addr_validate(&address)?;

            let user_payment_info = USER_PAYMENT_INFO.get(deps.storage, &user);

            if let Some(user_payment_info) = user_payment_info {
                to_binary(&user_payment_info)
            } else {
                Err(StdError::generic_err("No payment info is found"))
            }
        },
        QueryMsg::DealDetail { key, address, deal_id  } => {
            let config = CONFIG.load(deps.storage)?;
            let curr_admins = config.admins.clone();
            let curr_mods = MODERATORS.load(deps.storage)?;
            let active_deals = ACTIVE_DEALS.load(deps.storage)?;

            let is_valid = authenticate(deps, key, address.clone(), config.query_auth)?;

            if !is_valid {
                return Err(StdError::generic_err("Viewing key not match"));
            }

            let user = deps.api.addr_validate(&address)?;

            let mut selected_deal: Option<Deal> = None;
            let mut selected_payment_info: Option<PaymentInfo> = None;

            for deal in active_deals {
                if deal.deal_id == deal_id {
                    selected_deal = Some(deal);
                }
            }

            if selected_deal.is_none() {
                return Err(StdError::generic_err("No deal is found"));
            }

            let selected_deal_unwrap = selected_deal.unwrap();

            let deal_customer = selected_deal_unwrap.customer.clone();

            let permit = !curr_admins.contains(&user.clone())
                || !curr_mods.contains(&user.clone())
                || deal_customer.clone() == user.clone()
                || selected_deal_unwrap.dealer.clone() == user.clone();

            if permit {
                if selected_deal_unwrap.is_dealer_buy {
                    selected_payment_info = USER_PAYMENT_INFO.get(deps.storage, &deal_customer.clone());
                } else {
                    selected_payment_info = USER_PAYMENT_INFO.get(deps.storage, &selected_deal_unwrap.dealer.clone());
                }
            }

            if let Some(selected_payment_info) = selected_payment_info {
                to_binary(&QueryAnswer::DealDetail {
                    deal: selected_deal_unwrap,
                    payment_info: selected_payment_info
                })
            } else {
                Err(StdError::generic_err("No payment info is found"))
            }
        }, 
    }
}

pub fn authenticate(deps: Deps, key: String, address: String, query_auth: Contract) -> StdResult<bool> {
    let address = deps.api.addr_validate(&address)?;
    let querier = &deps.querier;

    let query_viewing_key = AuthQueryMsg::ValidateViewingKey { 
        user: address.clone(),
        key: key.clone()
    };
    let mut msg = to_binary(&query_viewing_key)?;
    let padding = RESPONSE_BLOCK_SIZE;

    space_pad(&mut msg.0, padding);
    let contract: Contract = query_auth.clone();

    let res: AuthQueryAnswer = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: contract.address.to_string(),
        msg,
        code_hash: contract.code_hash,
    }))?;

    match res {
        AuthQueryAnswer::ValidateViewingKey { is_valid } => {
            Ok(is_valid)
        }
        _ => Err(StdError::generic_err("Unauthorized")),
    }
}

pub fn space_pad(message: &mut Vec<u8>, block_size: usize) -> &mut Vec<u8> {
    let len = message.len();
    let surplus = len % block_size;
    if surplus == 0 {
        return message;
    }

    let missing = block_size - surplus;
    message.reserve(missing);
    message.extend(std::iter::repeat(b' ').take(missing));
    message
}