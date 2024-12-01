# Deal & Post

A P2P deal is implemented and can be described in the following schema

## Schema

```rust
pub struct Post {
    pub post_id: Uint128,
    pub is_dealer_buy: bool,
    pub amount: Uint128,
    pub min_amount: Uint128,
    pub settle_currency: String,
    pub settle_price: Uint128,
    pub dealer_deposit: bool,
    pub dealer: Addr,
    pub state: PostState,
}
```

| Property | Description |
|-------|-------------|
| post_id | unique ID per each post |
| is_dealer_buy | `True` if dealer is buying crypto from customer in the deal, vice versa |
| amount | Num. of token in this post |
| min_amount | Minimum num. of token a customer can trade with the dealer |
| settle_currency | The currency in this deal (e.g. USD) |
| settle_price | Price per each `deal_token` |
| dealer_deposit | `True` if dealer has already deposit crypto in this contract |
| dealer | Scrt address of the dealer |
| state | State of a post, see [State of a post](#state-of-a-post) below |

```rust
pub struct Deal {
    pub deal_id: Uint128,
    pub post_id: Uint128,
    pub is_dealer_buy: bool, // is dealer buying crypto
    pub amount: Uint128,
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
```

| Property | Description |
|-------|-------------|
| deal_id | unique ID per each deal |
| post_id | ID of the associated post |
| is_dealer_buy | `True` if dealer is buying crypto from customer in the deal, vice versa |
| amount | Num. of token in this deal, see [Calculation of transfer amount](#calculation-of-transfer-amount) below |
| settle_currency | The currency in this deal (e.g. USD) |
| settle_price | Price per each `deal_token` |
| dealer_deposit | `True` if dealer has already deposit crypto / made the bank transfer in this contract |
| customer_deposit | `True` if customer has already deposit crypto / made the bank transfer in this contract |
| dealer | Scrt address of the dealer |
| customer | Scrt address of the customer, only available when customer enters the deal |
| state | State of a deal, see [State of a deal](#state-of-a-deal) below |
| resolver | Scrt address of the moderator if a deal is closed by moderator, otherwise empty |
| expiry | Epoch time of the deal expiry. Counter-part can cancel/resolve the deal if time has passed the expiry |

## Calculation of transfer amount

P2P contract is pre-set with a `deal_token`, e.g. SILK token.

`amount` refers to how many `deal_token` is dealer buying / selling. Note that this value is `Uint128`. By following the snip-20 convention, this number needs to be divided by 1,000,000 to represent the correct value.

`settle_price` refers to the price per each `deal_token`. Note that this value is `Uint128`. By following the snip-20 convention, this number needs to be divided by 1,000,000 to represent the correct value.

(`amount` / 1,000,000 ) * ( `settle_price` / 1,000,000 ) = how much does counterpart need to wire transfer in the basis of the `settle_currency`. Or in other word, Wire transfer amount = `amount` * `settle_price` / 1_000_000_000_000.

For example, in the case when
- `is_dealer_buy` = `True`
- `amount` = `10_000_000`
- `settle_price` = `1_000_000`
- `settle_currency` = `USD`

Assuming `deal_token` is SLIK. It means that dealer is buying 10 SLIK at $1 per SILK.


## State of a post

| State | Description |
|-------|-------------|
| Open  | Post is now opened for customer to enter |
| PendDealerDeposit | Post is still pending dealer to deposit crypto before it can be entered by customer |

## State of a deal 

As parties proceed with the deal, the `state` of the deal will change accordingly. Depends on the `state`, there is a pre-defined list of actions that the parties can take to further proceed with the deal.

| State | Description |
|-------|-------------|
| PendCustomerDeposit | Happens after customer entering a deal in which dealer buys crypto from customer. Customer needs to deposit crypto first before dealer making wire transfer |
| PendDealerBankTransfer | Happens after customer depositing crypto to a deal in which dealer buys crypto. Dealer now needs to make wire transfer to customer |
| PendCustomerBankTransfer | Happens after customer enters a deal in which dealer sells crypto. Customer now needs to make wire transfer to dealer |
| PendDealerSignOff | After customer wire transfer. Dealer now needs to check and sign off the deal |
| PendCustomerSignOff | After dealer wire transfer. Customer now needs to check and sign off the deal |
| Dispute | Deal is now disputed by dealer/customer when wire transfer is missing despite the deal is marked "Pending___SignOff" |
| Resolve | Deal is finished and resolved |
| CancelAsDealerMissTransfer | Deal is cancelled by customer as dealer doesn't make wire transfer under specific time |
| CancelAsCustomerMissTransfer | Deal is cancelled by dealer as customer doesn't make wire transfer under specific time |
| CancelAsDispute | Deal is cancelled by mod as he/she thinks the wire transfer is invalid |
