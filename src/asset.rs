use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Api, ContractInfo, Deps, StdError, StdResult, Uint128};
use thiserror::Error;


pub const DEAL_EXPIRY_TIME: Uint128 = Uint128::new(21600);  // 6 hours
pub const DISPUTE_EXPIRY_TIME: Uint128 = Uint128::new(864000); // 10 days
pub const POST_EXPIRY_TIME: Uint128 = Uint128::new(432000); // 5 days
pub const COMMISSION_MULTIPLIER: Uint128 = Uint128::new(10000);

#[derive(Hash, Eq, Default)]
#[cw_serde]
pub struct RawContract {
    pub address: String,
    pub code_hash: String,
}

impl RawContract {
    #[allow(clippy::ptr_arg)]
    pub fn new(address: &String, code_hash: &String) -> Self {
        RawContract {
            address: address.clone(),
            code_hash: code_hash.clone(),
        }
    }

    /// Being deprecated in favor of `valid` which turns this into ContractInfo
    /// instead of a Contract (which we are getting rid of)
    pub fn into_valid(self, api: &dyn Api) -> StdResult<Contract> {
        let valid_addr = api.addr_validate(self.address.as_str())?;
        Ok(Contract::new(&valid_addr, &self.code_hash))
    }

    pub fn valid(self, api: &dyn Api) -> StdResult<ContractInfo> {
        let valid_addr = api.addr_validate(self.address.as_str())?;
        Ok(ContractInfo {
            address: valid_addr,
            code_hash: self.code_hash.clone(),
        })
    }
}

impl From<Contract> for RawContract {
    fn from(item: Contract) -> Self {
        RawContract {
            address: item.address.into(),
            code_hash: item.code_hash,
        }
    }
}

impl From<ContractInfo> for RawContract {
    fn from(item: ContractInfo) -> Self {
        RawContract {
            address: item.address.into(),
            code_hash: item.code_hash,
        }
    }
}

#[derive(Hash, Eq)]
#[cw_serde]
/// In the process of being deprecated for [cosmwasm_std::ContractInfo] so use that
/// instead when possible.
pub struct Contract {
    pub address: Addr,
    pub code_hash: String,
}

impl Default for Contract {
    fn default() -> Self {
        Self {
            address: Addr::unchecked(String::default()),
            code_hash: Default::default(),
        }
    }
}

impl Contract {
    #[allow(clippy::ptr_arg)]
    pub fn new(address: &Addr, code_hash: &String) -> Self {
        Contract {
            address: address.clone(),
            code_hash: code_hash.clone(),
        }
    }

    pub fn validate_new(deps: Deps, address: &str, code_hash: &String) -> StdResult<Self> {
        let valid_addr = deps.api.addr_validate(address)?;
        Ok(Contract::new(&valid_addr, code_hash))
    }
}

impl From<ContractInfo> for Contract {
    fn from(item: ContractInfo) -> Self {
        Contract {
            address: item.address,
            code_hash: item.code_hash,
        }
    }
}

impl Into<ContractInfo> for Contract {
    fn into(self) -> ContractInfo {
        ContractInfo {
            address: self.address,
            code_hash: self.code_hash,
        }
    }
}

#[derive(Hash, Eq)]
#[cw_serde]
pub enum PostState{
    Open,
    PendDealerDeposit  // snip
}

#[derive(Hash, Eq)]
#[cw_serde]
pub enum DealState {
    PendDealerBankTransfer,
    PendDealerSignOff,
    PendCustomerDeposit, // snip
    PendCustomerBankTransfer,
    PendCustomerSignOff,
    Dispute,
    Resolve,
    CancelAsDealerMissTransfer,
    CancelAsCustomerMissTransfer,
    CancelAsDispute
}

#[cw_serde]
pub enum DepositAction {
    Dealer {
        post_id: Uint128  // deposit to a post
    },
    Customer {
        deal_id: Uint128  // deposit to a deal
    },
}

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Governance exist and unauthoarized")]
    NotGovernanceSender {},

    #[error("User not yet register a payment info")]
    MissPaymentInfo {},

    #[error("Invalid Deal Token")]
    InvalidDealToken {},

    #[error("{0} cannot be divided by {1}")]
    DivideError(Uint128, Uint128),

    #[error("No Matching Deal with ID {0}")]
    NoMatchingDeal(Uint128),

    #[error("No Matching Post with ID {0}")]
    NoMatchingPost(Uint128),

    #[error("Unexpected Deal State")]
    UnexpectDealState,

    #[error("Unexpected Post State")]
    UnexpectPostState,

    #[error("You enter more than 1 Deal")]
    ConcurrentDealNotAllowed,

    #[error("Deposit required: {0}, but user only deposit {1}")]
    MismatchDepositAmount(Uint128, Uint128),

    #[error("Customer Mismatch")]
    MismatchCustomer,

    #[error("Dealer Mismatch")]
    MismatchDealer,

    #[error("No AD / Dealer token found in your address")]
    NoAdToken,

    #[error("Deal expiry datetime: {0}")]
    DealNotExpired(Uint128),

    #[error("The amount entered is less than dealer minimum amount requirement")]
    AmountLessThanDealerReq,

    #[error("The amount entered is more than the max amount selling in the post")]
    AmountMoreThanPost,

    #[error("Still active deal in this post")]
    ActiveDealExist,
}
