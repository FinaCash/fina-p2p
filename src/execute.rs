use cosmwasm_std::{
    from_binary, to_binary, Addr, Binary, CosmosMsg, DepsMut, Env, MessageInfo, Response, StdError, Uint128
};
use secret_toolkit::snip20::transfer_msg;

use crate::asset::{ContractError, DealState, DepositAction, PostState, RawContract, COMMISSION_MULTIPLIER, DEAL_EXPIRY_TIME, DISPUTE_EXPIRY_TIME, POST_EXPIRY_TIME};
use crate::contract::RESPONSE_BLOCK_SIZE;
use crate::msg::{ExecuteAnswer, ResponseStatus};
use crate::state::{Deal, PaymentInfo, Post, ACTIVE_DEALS, ACTIVE_POSTS, CONFIG, MAX_DEAL_ID, MAX_POST_ID, MODERATORS, PAST_DEALS, TOKEN_REVENUE, USER_PAYMENT_INFO};

pub fn update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    admins: Option<Vec<String>>,
    deal_commission: Option<Uint128>,
    query_auth: Option<RawContract>,
    governance: Option<RawContract>
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    // Authorization
    let sender = info.sender.clone();
    let curr_admins = config.admins.clone();
    if !curr_admins.contains(&sender.clone()) {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(admins) = admins {
        let admins_addr: Vec<Addr> = admins.iter()
            .map(|s| deps.api.addr_validate(s).unwrap())
            .collect();
        config.admins = admins_addr;
    }

    if let Some(deal_commission) = deal_commission {
        config.deal_commission = deal_commission;
    }

    if let Some(query_auth) = query_auth {
        config.query_auth = query_auth.into_valid(deps.api)?;
    }

    if let Some(governance) = governance {
        config.governance = Some(governance.into_valid(deps.api)?);
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(
        Response::new().set_data(to_binary(&ExecuteAnswer::UpdateConfig {
            status: ResponseStatus::Success,
        })?),
    )
}

pub fn update_deal_token(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    deal_token_a: RawContract,
    deal_token_b: RawContract,
    deal_token_c: RawContract,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    // Authorization
    if let Some(governance) = &config.governance {
        if info.sender != governance.address {
            return Err(ContractError::NotGovernanceSender {});
        }
    }

    // If governance doesn't exist, we will check admin
    let curr_admins = &config.admins;
    if !curr_admins.contains(&info.sender.clone())  {
        return Err(ContractError::Unauthorized {});
    }

    config.deal_token_a = deal_token_a.into_valid(deps.api)?;
    config.deal_token_b = deal_token_b.into_valid(deps.api)?;
    config.deal_token_c = deal_token_c.into_valid(deps.api)?;

    CONFIG.save(deps.storage, &config)?;

    Ok(
        Response::new().set_data(to_binary(&ExecuteAnswer::UpdateDealToken {
            status: ResponseStatus::Success,
        })?),
    )
}


pub fn add_moderator(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    mod_addr: String,
) -> Result<Response, ContractError> {
    let moderator = deps.api.addr_validate(&mod_addr).unwrap();
    let mut curr_mods = MODERATORS.load(deps.storage)?;
    
    let config = CONFIG.load(deps.storage)?;

    // Authorization
    if let Some(governance) = &config.governance {
        if info.sender != governance.address {
            return Err(ContractError::NotGovernanceSender {});
        }
    }

    // If governance doesn't exist, we will check admin
    let curr_admins = &config.admins;
    if !curr_admins.contains(&info.sender.clone())  {
        return Err(ContractError::Unauthorized {});
    }

    curr_mods.push(moderator);

    MODERATORS.save(deps.storage, &curr_mods)?;

    Ok(
        Response::new().set_data(to_binary(&ExecuteAnswer::AddModerator {
            status: ResponseStatus::Success,
        })?),
    )
}


pub fn remove_moderator(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    mod_addr: String,
) -> Result<Response, ContractError> {
    let moderator = deps.api.addr_validate(&mod_addr).unwrap();
    let mut curr_mods = MODERATORS.load(deps.storage)?;
    
    let config = CONFIG.load(deps.storage)?;

    // Authorization
    if let Some(governance) = &config.governance {
        if info.sender != governance.address {
            return Err(ContractError::NotGovernanceSender {});
        }
    }

    // If governance doesn't exist, we will check admin
    let curr_admins = &config.admins;
    if !curr_admins.contains(&info.sender.clone())  {
        return Err(ContractError::Unauthorized {});
    }

    // remove the moderator
    curr_mods.retain(|s| s != &moderator);

    MODERATORS.save(deps.storage, &curr_mods)?;

    Ok(
        Response::new().set_data(to_binary(&ExecuteAnswer::AddModerator {
            status: ResponseStatus::Success,
        })?),
    )
}


pub fn register_user_payment_info(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    method: String,
    detail: String
) -> Result<Response, ContractError> {
    let user_payment_info = PaymentInfo {
        method,
        detail
    };
    USER_PAYMENT_INFO.insert(deps.storage, &info.sender, &user_payment_info)?;
    Ok(
        Response::new().set_data(to_binary(&ExecuteAnswer::RegisterPaymentInfo {
            status: ResponseStatus::Success,
        })?),
    )
}

pub fn receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    _sender: Addr,
    from: Addr,
    amount: Uint128,
    msg: Option<Binary>
) -> Result<Response, ContractError> {
    // info.sender is token contract
    // from is the user
    let now = Uint128::new(env.block.time.seconds() as u128);

    match msg {
        Some(m) => match from_binary(&m)? {
            DepositAction::Customer { deal_id } => {
                let mut active_deals = ACTIVE_DEALS.load(deps.storage)?;

                match active_deals.iter_mut().find(|x| x.deal_id == deal_id) {
                    Some(deal) => {
                        // check deal state is correct
                        if deal.state != DealState::PendCustomerDeposit {
                            return Err(ContractError::UnexpectDealState {});
                        }

                        // check deposit amt
                        if deal.amount != amount {
                            return Err(ContractError::MismatchDepositAmount {0: deal.amount, 1: amount});
                        }

                        // check if customer is indeed the customer shown in the deal
                        let customer_should_be = &deal.customer;
                        if customer_should_be != &from {
                            return Err(ContractError::MismatchCustomer {});
                        }

                        // check if user is paying the correct snip token for deal making
                        if info.sender != deal.deal_token.address {
                            return Err(ContractError::InvalidDealToken {});
                        }

                        deal.customer_deposit = true;
                        deal.state = DealState::PendDealerBankTransfer;
                        deal.expiry = Some(now + DEAL_EXPIRY_TIME);

                        ACTIVE_DEALS.save(deps.storage, &active_deals.clone())?;

                        Ok(
                            Response::new().set_data(to_binary(&ExecuteAnswer::CustomerDeposit {
                                status: ResponseStatus::Success,
                            })?)
                        )
                    },
                    None => {
                        return Err(ContractError::NoMatchingDeal { 0: deal_id });
                    }
                }
            }
            DepositAction::Dealer { post_id } => {
                let mut active_posts = ACTIVE_POSTS.load(deps.storage)?;

                match active_posts.iter_mut().find(|x| x.post_id == post_id) {
                    Some(post) => {
                        // check deal state is correct
                        if post.state != PostState::PendDealerDeposit {
                            return Err(ContractError::UnexpectPostState {});
                        }

                        // check deposit amt
                        if post.amount != amount {
                            return Err(ContractError::MismatchDepositAmount {0: post.amount, 1: amount});
                        }

                        // check if dealer is indeed the dealer shown in the deal
                        let dealer_should_be = &post.dealer;
                        if dealer_should_be != &from {
                            return Err(ContractError::MismatchDealer {});
                        }

                        // check if user is paying the correct snip token for deal making
                        let deal_token = post.deal_token.address.clone();
                        if info.sender != deal_token {
                            return Err(ContractError::InvalidDealToken {});
                        }

                        post.dealer_deposit = true;
                        post.state = PostState::Open;

                        // save deal
                        ACTIVE_POSTS.save(deps.storage, &active_posts.clone())?;

                        Ok(
                            Response::new().set_data(to_binary(&ExecuteAnswer::DealerDeposit {
                                status: ResponseStatus::Success,
                                sender: info.sender,
                                deposit_token: deal_token,
                            })?)
                        )
                    },
                    None => {
                        return Err(ContractError::NoMatchingPost { 0: post_id });
                    }
                }
            }
        },
        None => {
            return Err(ContractError::Std(StdError::generic_err("No action provided")));
        }
    }
}

pub fn add_post(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    is_dealer_buy: bool,  // is dealer buying crypto or selling crypto.
    deal_token: RawContract,
    amount: Uint128,  // number of crypto, 1_000_000 = 1 crypto
    min_amount: Uint128,
    settle_currency: String,  // currency of the trade
    settle_price: Uint128,
) -> Result<Response, ContractError> {
    // let support_currencies = vec![
    //     "HKD".to_string(),
    //     "USD".to_string(),
    //     "EUR".to_string()
    // ];

    // if !support_currencies.contains(&settle_currency.clone()) {
    //     return Err(ContractError::Std(StdError::generic_err("Unsupport currency")));
    // }

    if amount < Uint128::zero() {
        return Err(ContractError::Std(StdError::generic_err("Amount cannot be lower than 0")));
    }

    if min_amount < Uint128::zero() {
        return Err(ContractError::Std(StdError::generic_err("Min amount cannot be lower than 0")));
    }

    if settle_price < Uint128::zero() {
        return Err(ContractError::Std(StdError::generic_err("Settle price cannot be lower than 0")));
    }

    if !USER_PAYMENT_INFO.contains(deps.storage, &info.sender) {
        return Err(ContractError::MissPaymentInfo {});
    }

    let mut active_posts = ACTIVE_POSTS.load(deps.storage)?;

    // update post id
    let new_id = MAX_POST_ID.load(deps.storage)? + Uint128::new(1);
    MAX_POST_ID.save(deps.storage, &new_id)?;

    let mut init_state = PostState::Open;

    // if dealer is selling crypto, dealer needs to deposit the crypto first
    if !is_dealer_buy {
        init_state = PostState::PendDealerDeposit;
    }

    // calculate post expiry
    let now = Uint128::new(env.block.time.seconds() as u128);
    let expiry = now + POST_EXPIRY_TIME;

    let deal_token_valid = deal_token.into_valid(deps.api)?;

    let config = CONFIG.load(deps.storage)?;

    if deal_token_valid != config.deal_token_a && deal_token_valid != config.deal_token_b && deal_token_valid != config.deal_token_c {
        return Err(ContractError::InvalidDealToken {});
    }

    active_posts.push(Post {
        post_id: new_id.clone(),
        is_dealer_buy: is_dealer_buy,
        deal_token: deal_token_valid,
        amount: amount,
        min_amount: min_amount,
        settle_currency: settle_currency,
        settle_price: settle_price,
        dealer_deposit: false,
        dealer: info.sender.clone(),
        state: init_state,
        expiry: expiry,
    });

    ACTIVE_POSTS.save(deps.storage, &active_posts)?;

    Ok(
        Response::new().set_data(to_binary(&ExecuteAnswer::AddPost {
            status: ResponseStatus::Success,
            post_id: new_id
        })?)
    )
}

pub fn cancel_post(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    post_id: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let mut active_posts = ACTIVE_POSTS.load(deps.storage)?;
    const SUPPORT_POST_STATES: [PostState;  2] = [
        PostState::Open,
        PostState::PendDealerDeposit,
    ];

    let mut cosmos_msg: Option<CosmosMsg> = None;

    match active_posts.iter_mut().find(|x| x.post_id == post_id) {
        Some(post) => {

            // check deal state is correct
            if !SUPPORT_POST_STATES.contains(&post.state) {
                return Err(ContractError::UnexpectDealState {});
            }

            // check if dealer in the post matches sender
            let dealer_should_be = &post.dealer;
            if dealer_should_be != &info.sender {
                return Err(ContractError::MismatchDealer {});
            }

            // If deal is open, it could mean dealer has already deposit crypto 
            // if he wants to sell, we need to refund
            if post.dealer_deposit {
                // get the deal token detail first
                let deal_token = post.deal_token.clone();

                cosmos_msg = Some(transfer_msg(
                    post.dealer.clone().into_string(),
                    post.amount,
                    None,
                    None, 
                    RESPONSE_BLOCK_SIZE,
                    deal_token.code_hash,
                    deal_token.address.into_string()
                )?);
            }
        },
        None => {
            return Err(ContractError::NoMatchingPost { 0: post_id });
        }
    }

    // remove post
    active_posts.retain(|x| x.post_id != post_id);
    ACTIVE_POSTS.save(deps.storage, &active_posts)?;

    // return token if dealer already deposit
    if let Some(cosmos_msg) = cosmos_msg {
        Ok(Response::new()
            .add_message(cosmos_msg)
            .set_data(to_binary(&ExecuteAnswer::CancelPost {
                status: ResponseStatus::Success,
                post_id: post_id
        })?))
    } else {
        Ok(Response::new().set_data(to_binary(&ExecuteAnswer::CancelPost {
            status: ResponseStatus::Success,
            post_id: post_id
        })?))
    }
}

pub fn enter_deal(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    post_id: Uint128,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let now = Uint128::new(env.block.time.seconds() as u128);

    // Customer to accept a post and open a deal with dealer
    let mut active_posts = ACTIVE_POSTS.load(deps.storage)?;
    let mut active_deals = ACTIVE_DEALS.load(deps.storage)?;

    const SUPPORT_STATES: [PostState;  1] = [
        PostState::Open
    ];

    // Check if customer has already register for payment info
    let user_payment_info = USER_PAYMENT_INFO.get(deps.storage, &info.sender.clone());

    if user_payment_info.is_none() {
        return Err(ContractError::MissPaymentInfo {});
    }

    match active_posts.iter_mut().find(|x| x.post_id == post_id) {
        Some(post) => {
            // list of things to check before deal is created
            // check post state is correct
            if !SUPPORT_STATES.contains(&post.state) {
                return Err(ContractError::UnexpectPostState {});
            }

            // check if deal amount proposed by customer is less than min amount set by dealer
            if amount < post.min_amount {
                return Err(ContractError::AmountLessThanDealerReq {});
            }

            // check if deal amount proposed by customer is more than the remaining post amount
            if amount > post.amount {
                return Err(ContractError::AmountMoreThanPost {});
            }

            // deducing amount from the post
            let new_post_amount = post.amount - amount;
            post.amount = new_post_amount;

            // create a new deal
            // update deal id
            let new_id = MAX_DEAL_ID.load(deps.storage)? + Uint128::new(1);
            MAX_DEAL_ID.save(deps.storage, &new_id)?;

            // deal state
            let new_deal_state: DealState;

            if post.is_dealer_buy {
                new_deal_state = DealState::PendCustomerDeposit;
            } else {
                new_deal_state = DealState::PendCustomerBankTransfer;
            }

            // deal expiry
            let deal_expiry = Some(now + DEAL_EXPIRY_TIME);

            active_deals.push(Deal {
                deal_id: new_id.clone(),
                post_id: post_id.clone(),
                is_dealer_buy: post.is_dealer_buy.clone(),
                deal_token: post.deal_token.clone(),
                amount: amount,
                settle_currency: post.settle_currency.clone(),
                settle_price: post.settle_price.clone(),
                dealer_deposit: post.dealer_deposit.clone(),
                customer_deposit: false,
                dealer: post.dealer.clone(),
                customer: info.sender.clone(),
                state: new_deal_state,
                resolver: None,
                expiry: deal_expiry
            });

            // commit the change on deal + post
            ACTIVE_POSTS.save(deps.storage, &active_posts.clone())?;
            ACTIVE_DEALS.save(deps.storage, &active_deals.clone())?;

            Ok(
                Response::new().set_data(to_binary(&ExecuteAnswer::DealStageProcess {
                    status: ResponseStatus::Success,
                    deal_id: new_id
                })?)
            )
        },
        None => {
            return Err(ContractError::NoMatchingPost { 0: post_id });
        }
    }
}

pub fn confirm_bank_transfer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    deal_id: Uint128,
) -> Result<Response, ContractError> {
    let now = Uint128::new(env.block.time.seconds() as u128);

    let mut active_deals = ACTIVE_DEALS.load(deps.storage)?;
    const SUPPORT_STATES: [DealState;  2] = [
        DealState::PendCustomerBankTransfer,
        DealState::PendDealerBankTransfer
    ];

    match active_deals.iter_mut().find(|x| x.deal_id == deal_id) {
        Some(deal) => {

            // check deal state is correct
            if !SUPPORT_STATES.contains(&deal.state) {
                return Err(ContractError::UnexpectDealState {});
            }

            if &deal.state == &DealState::PendCustomerBankTransfer {
                // customer bank transfer should be confirmed by customer
                let sender_should_be = &deal.customer.clone();
                if sender_should_be != &info.sender {
                    return Err(ContractError::MismatchCustomer {});
                }

                deal.customer_deposit = true;
                deal.state = DealState::PendDealerSignOff;
            } else {
                // dealer bank transfer should be confirmed by dealer
                let sender_should_be = &deal.dealer;
                if sender_should_be != &info.sender {
                    return Err(ContractError::MismatchDealer {});
                }

                deal.dealer_deposit = true;
                deal.state = DealState::PendCustomerSignOff;
            }

            deal.expiry = Some(now + DEAL_EXPIRY_TIME);

            // save deal
            ACTIVE_DEALS.save(deps.storage, &active_deals.clone())?;

            Ok(
                Response::new().set_data(to_binary(&ExecuteAnswer::DealStageProcess {
                    status: ResponseStatus::Success,
                    deal_id: deal_id
                })?)
            )
        },
        None => {
            return Err(ContractError::NoMatchingDeal { 0: deal_id });
        }
    }
}

pub fn dispute_deal(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    deal_id: Uint128,
) -> Result<Response, ContractError> {
    let now = Uint128::new(env.block.time.seconds() as u128);

    let mut active_deals = ACTIVE_DEALS.load(deps.storage)?;
    const SUPPORT_STATES: [DealState;  2] = [
        DealState::PendCustomerSignOff,
        DealState::PendDealerSignOff
    ];

    match active_deals.iter_mut().find(|x| x.deal_id == deal_id) {
        Some(deal) => {

            // check deal state is correct
            if !SUPPORT_STATES.contains(&deal.state) {
                return Err(ContractError::UnexpectDealState {});
            }

            if &deal.state == &DealState::PendCustomerSignOff {
                // customer bank transfer should be confirmed by customer
                let sender_should_be = &deal.customer.clone();
                if sender_should_be != &info.sender {
                    return Err(ContractError::MismatchCustomer {});
                }
            } else {
                // dealer bank transfer should be confirmed by dealer
                let sender_should_be = &deal.dealer;
                if sender_should_be != &info.sender {
                    return Err(ContractError::MismatchDealer {});
                }
            }

            deal.state = DealState::Dispute;
            deal.expiry = Some(now + DISPUTE_EXPIRY_TIME);

            // save deal
            ACTIVE_DEALS.save(deps.storage, &active_deals.clone())?;

            Ok(
                Response::new().set_data(to_binary(&ExecuteAnswer::DealStageProcess {
                    status: ResponseStatus::Success,
                    deal_id: deal_id
                })?)
            )
        },
        None => {
            return Err(ContractError::NoMatchingDeal { 0: deal_id });
        }
    }
}

pub fn resolve_deal(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    deal_id: Uint128,
) -> Result<Response, ContractError> {
    let now = Uint128::new(env.block.time.seconds() as u128);

    const SUPPORT_STATES: [DealState;  3] = [
        DealState::PendCustomerSignOff,
        DealState::PendDealerSignOff,
        DealState::Dispute
    ];

    let config = CONFIG.load(deps.storage)?;
    let mut active_deals = ACTIVE_DEALS.load(deps.storage)?;
    let mut past_deals = PAST_DEALS.load(deps.storage)?;

    let deal_post_id: Uint128;

    let curr_mods = MODERATORS.load(deps.storage)?;

    let cosmos_msg: Option<CosmosMsg>;
    let commission: Uint128;

    match active_deals.iter_mut().find(|x| x.deal_id == deal_id) {
        Some(deal) => {

            // check deal state is correct
            if !SUPPORT_STATES.contains(&deal.state) {
                return Err(ContractError::UnexpectDealState {});
            }

            deal_post_id = deal.post_id.clone();

            // normal customer sign off [Dealer buying crypto from Customer]
            if &deal.state == &DealState::PendCustomerSignOff {
                // dealer bank transfer should be confirmed by customer before expiry
                // but can also be resolved by dealer after expiry
                let customer = deal.customer.clone();

                if &deal.expiry.unwrap() > &now {
                    let sender_should_be = &customer;
                    if sender_should_be != &info.sender {
                        return Err(ContractError::MismatchCustomer {});
                    }
                } else {
                    let senders_should_be = vec![
                        &customer,
                        &deal.dealer,
                    ];

                    if !senders_should_be.contains(&&info.sender) {
                        return Err(ContractError::Unauthorized {})
                    }
                }

                commission = calculate_commission(deal.amount.clone(), config.deal_commission.clone());
                let payout = deal.amount.clone() - &commission;

                let deal_token = deal.deal_token.clone();

                cosmos_msg = Some(transfer_msg(
                    deal.dealer.clone().into_string(),
                    payout,
                    None,
                    None, 
                    RESPONSE_BLOCK_SIZE,
                    deal_token.code_hash,
                    deal_token.address.into_string()
                )?);

            // normal dealer sign off [Dealer selling crypto to Customer]
            } else if &deal.state == &DealState::PendDealerSignOff {
                // customer bank transfer should be confirmed by dealer
                // but can also be resolved by customer after expiry
                let customer = deal.customer.clone();

                if &deal.expiry.unwrap() > &now {
                    let sender_should_be = &deal.dealer;
                    if sender_should_be != &info.sender {
                        return Err(ContractError::MismatchDealer {});
                    }
                } else {
                    let senders_should_be = vec![
                        &customer,
                        &deal.dealer,
                    ];

                    if !senders_should_be.contains(&&info.sender) {
                        return Err(ContractError::Unauthorized {})
                    }
                }

                commission = calculate_commission(deal.amount.clone(), config.deal_commission.clone());
                let payout = deal.amount.clone() - &commission;

                let deal_token = deal.deal_token.clone();

                cosmos_msg = Some(transfer_msg(
                    customer.into_string(),
                    payout,
                    None,
                    None, 
                    RESPONSE_BLOCK_SIZE,
                    deal_token.code_hash,
                    deal_token.address.into_string()
                )?);

            // Dispute, and admin decides to still resolve the case
            } else {
                if !curr_mods.contains(&info.sender.clone())  {
                    return Err(ContractError::Unauthorized {});
                }

                let receiver = if deal.is_dealer_buy
                    { deal.dealer.clone() } 
                    else { deal.customer.clone() };

                commission = calculate_commission(deal.amount.clone(), config.deal_commission.clone());
                let payout = deal.amount.clone() - &commission;

                let deal_token = deal.deal_token.clone();

                cosmos_msg = Some(transfer_msg(
                    receiver.into_string(),
                    payout,
                    None,
                    None, 
                    RESPONSE_BLOCK_SIZE,
                    deal_token.code_hash,
                    deal_token.address.into_string()
                )?);
            }

            deal.state = DealState::Resolve;
            deal.resolver = Some(info.sender.clone());

            // archive deal into past deals
            past_deals.push(deal.clone());
            PAST_DEALS.save(deps.storage, &past_deals)?;

            // add revenue
            let new_revenue = TOKEN_REVENUE.load(deps.storage)? + &commission;
            TOKEN_REVENUE.save(deps.storage, &new_revenue)?;
        },
        None => {
            return Err(ContractError::NoMatchingDeal { 0: deal_id });
        }
    }

    // remove deal
    active_deals.retain(|x| x.deal_id != deal_id);
    ACTIVE_DEALS.save(deps.storage, &active_deals)?;

    // change and remove post if its zero balance
    let mut is_remove_post = false;
    let mut active_posts = ACTIVE_POSTS.load(deps.storage)?;
    match active_posts.iter_mut().find(|x| x.post_id == deal_post_id) {
        Some(post) => {
            if post.amount == Uint128::zero() {
                is_remove_post = true;
            }
        },
        None => {
            is_remove_post = false;
        }
    }

    if is_remove_post {
        active_posts.retain(|x| x.post_id != deal_post_id);
        ACTIVE_POSTS.save(deps.storage, &active_posts)?;
    }

    Ok(Response::new().add_message(cosmos_msg.unwrap()))
}

fn calculate_commission(amount: Uint128, comm_bps: Uint128) -> Uint128 {
    (amount * comm_bps) / COMMISSION_MULTIPLIER
}

pub fn cancel_deal(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    deal_id: Uint128,
) -> Result<Response, ContractError> {
    const SUPPORT_STATES: [DealState;  4] = [
        DealState::PendCustomerDeposit,
        DealState::PendDealerBankTransfer,
        DealState::PendCustomerBankTransfer,
        DealState::Dispute
    ];

    let now = Uint128::new(env.block.time.seconds() as u128);

    let config = CONFIG.load(deps.storage)?;
    let mut active_deals = ACTIVE_DEALS.load(deps.storage)?;
    let mut past_deals = PAST_DEALS.load(deps.storage)?;

    let curr_mods = MODERATORS.load(deps.storage)?;
    // let deal_post_id: Uint128;
    // let deal_post_amount: Uint128;
    let mut cosmos_msg: Option<CosmosMsg> = None;

    match active_deals.iter_mut().find(|x| x.deal_id == deal_id) {
        Some(deal) => {

            // check deal state is correct
            if !SUPPORT_STATES.contains(&deal.state) {
                return Err(ContractError::UnexpectDealState {});
            }

            // deal_post_id = deal.post_id.clone();
            // deal_post_amount = deal.amount.clone();

            if &deal.state == &DealState::PendCustomerDeposit {
                if &deal.customer.clone() != &info.sender && &deal.dealer != &info.sender {
                    return Err(ContractError::Unauthorized {});
                }

                // it also has to be an expired deal for dealer to cancel it
                // intutively, customer has no action for too long
                if &deal.expiry.unwrap() > &now {
                    return Err(ContractError::DealNotExpired(deal.expiry.unwrap()));
                }

                deal.state = DealState::CancelAsCustomerMissTransfer;

                // archive deal into past deals
                past_deals.push(deal.clone());
                PAST_DEALS.save(deps.storage, &past_deals)?;

            } else if &deal.state == &DealState::PendCustomerBankTransfer {
                if &deal.customer.clone() != &info.sender && &deal.dealer != &info.sender {
                    return Err(ContractError::Unauthorized {});
                }

                // it also has to be an expired deal for dealer to cancel it
                // intutively, customer has no action for too long
                if &deal.expiry.unwrap() > &now {
                    return Err(ContractError::DealNotExpired(deal.expiry.unwrap()));
                }

                deal.state = DealState::CancelAsCustomerMissTransfer;

                let deal_token = deal.deal_token.clone();

                // the deal amount is already deposited by dealer, need to refund it
                // Refund to dealer
                cosmos_msg = Some(transfer_msg(
                    deal.dealer.clone().into_string(),
                    deal.amount,
                    None,
                    None, 
                    RESPONSE_BLOCK_SIZE,
                    deal_token.code_hash,
                    deal_token.address.into_string()
                )?);

                // archive deal into past deals
                past_deals.push(deal.clone());
                PAST_DEALS.save(deps.storage, &past_deals)?;

            } else if &deal.state == &DealState::PendDealerBankTransfer {
                // if get to here it means Customer has paid crypto to the pool
                // but dealer has not yet execute the wire transfer
                // only customer is allowed to cancel the deal from here
                // dealer should have fulfill the obligation
                if &deal.customer.clone() != &info.sender {
                    return Err(ContractError::Unauthorized {});
                }

                // it also has to be an expired deal for customer to cancel it
                // intutively, dealer has no action for too long
                if &deal.expiry.unwrap() > &now {
                    return Err(ContractError::DealNotExpired(deal.expiry.unwrap()));
                }

                let deal_token = deal.deal_token.clone();

                // Refund to customer
                cosmos_msg = Some(transfer_msg(
                    deal.customer.clone().into_string(),
                    deal.amount,
                    None,
                    None, 
                    RESPONSE_BLOCK_SIZE,
                    deal_token.code_hash,
                    deal_token.address.into_string()
                )?);

                deal.state = DealState::CancelAsDealerMissTransfer;

                // archive deal into past deals
                past_deals.push(deal.clone());
                PAST_DEALS.save(deps.storage, &past_deals)?;

            } else if &deal.state == &DealState::Dispute {
                // Dispute + cancel = refund to the customer/dealer

                if !curr_mods.contains(&info.sender.clone()) {
                    return Err(ContractError::Unauthorized {});
                }

                // refund
                let recipient = if deal.is_dealer_buy {
                    deal.customer.clone().into_string()
                } else { deal.dealer.clone().into_string()};

                let deal_token = deal.deal_token.clone();

                cosmos_msg =  Some(transfer_msg(
                    recipient,
                    deal.amount,
                    None,
                    None, 
                    RESPONSE_BLOCK_SIZE,
                    deal_token.code_hash,
                    deal_token.address.into_string()
                )?);
    
                deal.state = DealState::CancelAsDispute;
                deal.resolver = Some(info.sender.clone());

                // archive deal into past deals
                past_deals.push(deal.clone());
                PAST_DEALS.save(deps.storage, &past_deals)?;
            }
        },
        None => {
            return Err(ContractError::NoMatchingDeal { 0: deal_id });
        }
    }
    // remove the deal
    active_deals.retain(|x| x.deal_id != deal_id);
    ACTIVE_DEALS.save(deps.storage, &active_deals.clone())?;

    // Diable the function to revert the amount back to the post
    // As dealer can always cancel the post, dangerous to assume the post still exist
    // Also, locking the post is not ideal as price can change frequently
    // // top up the deal amount back to the post
    // let mut active_posts = ACTIVE_POSTS.load(deps.storage)?;

    // match active_posts.iter_mut().find(|x| x.post_id == deal_post_id) {
    //     Some(post) => {
    //         let new_post_amount = post.amount + deal_post_amount;
    //         post.amount = new_post_amount;

    //         ACTIVE_POSTS.save(deps.storage, &active_posts.clone())?;
    //     },
    //     None => {
    //         return Err(ContractError::NoMatchingPost { 0: deal_post_id });
    //     }
    // }

    if let Some(cosmos_msg) = cosmos_msg {
        Ok(Response::new()
            .add_message(cosmos_msg)
            .set_data(to_binary(&ExecuteAnswer::DealStageProcess {
                status: ResponseStatus::Success,
                deal_id: deal_id
        })?))
    } else {
        Ok(Response::new()
            .set_data(to_binary(&ExecuteAnswer::DealStageProcess {
                status: ResponseStatus::Success,
                deal_id: deal_id
            })?)
        )
    }
}

pub fn emergency_withdraw(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    deal_id: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut active_deals = ACTIVE_DEALS.load(deps.storage)?;

    let curr_admins = config.admins;

    if !curr_admins.contains(&info.sender.clone()) {
        return Err(ContractError::Unauthorized {});
    }

    let cosmos_msg: CosmosMsg;

    match active_deals.iter_mut().find(|x| x.deal_id == deal_id) {
        Some(deal) => {

            if (deal.is_dealer_buy) && !deal.customer_deposit {
                return Err(ContractError::Std(StdError::generic_err("No Amount to withdraw"))); 
            }

            if !(deal.is_dealer_buy) && !deal.dealer_deposit {
                return Err(ContractError::Std(StdError::generic_err("No Amount to withdraw"))); 
            }

            let deal_token = deal.deal_token.clone();

            cosmos_msg = transfer_msg(
                info.sender.into_string(),
                deal.amount,
                None,
                None, 
                RESPONSE_BLOCK_SIZE,
                deal_token.code_hash,
                deal_token.address.into_string()
            )?;
        },
        None => {
            return Err(ContractError::NoMatchingDeal { 0: deal_id });
       }
    }

    // remove deal
    active_deals.retain(|x| x.deal_id != deal_id);
    ACTIVE_DEALS.save(deps.storage, &active_deals)?;

    Ok(Response::new().add_message(cosmos_msg))
}

pub fn admin_delete_deal(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    deal_id: Uint128,
) -> Result<Response, ContractError> {
    let mut active_deals = ACTIVE_DEALS.load(deps.storage)?;
    let config = CONFIG.load(deps.storage)?;
    let curr_admins = config.admins.clone();

    if !curr_admins.contains(&info.sender.clone()) {
        return Err(ContractError::Unauthorized {});
    }

    match active_deals.iter_mut().find(|x| x.deal_id == deal_id) {
        Some(deal) => {
            active_deals.retain(|x| x.deal_id != deal_id);
            ACTIVE_DEALS.save(deps.storage, &active_deals.clone())?;   
        },
        None => {
            return Err(ContractError::NoMatchingDeal { 0: deal_id });
       }
    }

    Ok(Response::new()
       .set_data(to_binary(&ExecuteAnswer::DealStageProcess {
           status: ResponseStatus::Success,
           deal_id: deal_id
       })?)
   )
}

pub fn get_commission(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut total_revenue = TOKEN_REVENUE.load(deps.storage)?;

    let curr_admins = config.admins;

    if !curr_admins.contains(&info.sender.clone()) {
        return Err(ContractError::Unauthorized {});
    }

    let cosmos_msg = transfer_msg(
        info.sender.into_string(),
        total_revenue,
        None,
        None, 
        RESPONSE_BLOCK_SIZE,
        config.deal_token_a.code_hash,
        config.deal_token_a.address.into_string()
    )?;

    total_revenue = Uint128::zero();
    TOKEN_REVENUE.save(deps.storage, &total_revenue.clone())?;

    Ok(Response::new().add_message(cosmos_msg))
}