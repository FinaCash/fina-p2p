# [Internal] Contract

For internal review only, to avoid exploit.

## List of TESTNET contract address

| Key                   | Value                                   |
|-------------------------|--------------------------------------------|
| query_auth             | secret1ww9r0q02x0altkdyya3f9ygnc53qc2asc2tc5g |
| fina_token             | secret1etrc3h558yesk25j5pu835f9s84wjvjzeed6e9 |
| p2p_contract           | secret1g3qrmfsvrnqnfs2cjvpkukmpz9ce624hdn7np9 |

## P2P configuration

```json
{
   "config":{
      "config":{
         "admins":[
            "secret1t6v5pv9jgddv4t43gdzz348desfsy9727ngfcc"
         ],
         "deal_commission":"1",
         "deal_token":{
            "address":"secret1etrc3h558yesk25j5pu835f9s84wjvjzeed6e9",
            "code_hash":"681c588acfa0d3ccaa4c6a798813d4d1bf3f719159ba3bd03c0c5e5a1e26a05e"
         },
         "query_auth":{
            "address":"secret1ww9r0q02x0altkdyya3f9ygnc53qc2asc2tc5g",
            "code_hash":"260dedf9de44110f3ab1ae528c8f27ed153b2c4d6a0ace75b4c6c8f6be415ae4"
         },
         "governance":null
      }
   }
}
```

| Key                   | Description                                   |
|-------------------------|--------------------------------------------|
| admin             | Address of the admin |
| deal_commission   | expressed in bps. For example, when commission == `1`, it means we will take 0.01% of the entire deal amount as commission |
| deal_token        | Address of the snip-20 token that users are dealing within this p2p contract |
| query_auth        | Address of the query auth contract that provides privacy feature |
| governance        | Address of the governance contract. When this is available, it will take over execution function that's control by admin (See [below](#admin-execution-functions)) |

## User setup to be ready for p2p

1. Set up viewing key in the query auth contract. For them to authenticate themselves later to be able to view their payment detail.

```bash
Q_VIEWING_KEY="api_key_fWFwJIoxqgtqG/FWLqvxDzYVheENhBZFY4ZiIfRhjU8="

secretcli tx compute execute "$QUERY_AUTH_ADDRESS" \
	'{
		"set_viewing_key": {"key": "'"$Q_VIEWING_KEY"'"}
	}' --from investor1 --fees 2500uscrt -y

```

```
"output_data_as_string": "{\"set_viewing_key\":{\"status\":\"success\"}} 
```

2. Set up payment info in the p2p contract. For that to be expose to counterpart later when they enter the deal

```bash
PAYMENT_METHOD="fps"
PAYMENT_DETAIL="number: 123456"

secretcli tx compute execute "$P2P_CONTRACT" \
	'{
		"register_payment_info": {
			"method": "'"$PAYMENT_METHOD"'",
			"detail": "'"$PAYMENT_DETAIL"'"
		}
	}' --from investor1 --fees 2500uscrt -y
```

```
"output_data_as_string": "{\"register_payment_info\":{\"status\":\"success\"}}
```

## Dealer execution to start a deal

### Dealer adds a post to the contract

Note Dealer needs to register his payment info first before he is able to add a post.

*Dealer selling crypto*

```bash
IS_BUY=false
AMOUNT=3000000
MIN_AMOUNT=1000000
SETTLE_CURRENCY="USD"
SETTLE_PRICE=1000000

secretcli tx compute execute "$P2P_CONTRACT" \
	'{
		"add_post": {
			"is_dealer_buy": '"$IS_BUY"',
			"amount": "'"$AMOUNT"'",
			"min_amount": "'"$MIN_AMOUNT"'",
			"settle_currency": "'"$SETTLE_CURRENCY"'",
			"settle_price": "'"$SETTLE_PRICE"'"
		}
	}' --from investor1 --fees 2500uscrt -y
```

Output includes a `post_id`.
```
"output_data_as_string": "{\"add_post\":{\"status\":\"success\",\"post_id\":\"1\"}} 
```

Assuming that `deal_token` is set to SILK. This message demostrates dealer is adding a post to sell 3 SLIK, at 1USD per SLIK.

Post state is now `PendDealerDeposit`. Since dealer is selling crypto, he is required to deposit crypto into the smart contract first. Depositing crypto is same as sending snip-20 token, but with a specific message

This message represents a dealer deposit to deal with `post_id` equals to 1. Note that the amount sent must match the amount of the post, otherwise it will fail.

```bash
MESSAGE='{
	"dealer": {
		"post_id": "1"
	}
}'

BASE64_MSG=$(echo -n "$MESSAGE" | base64)
TOTAL_TOKEN=3000000

secretcli tx compute execute "$FINA_CONTRACT" \
	'{
		"send": {
			"recipient": "'"$P2P_CONTRACT"'",
			"amount": "'"$TOTAL_TOKEN"'",
			"msg": "'"$BASE64_MSG"'"
		}
	}' --from investor1 --fees 2500uscrt -y
```

Deal state is now `Open`.

*Dealer buying crypto*

```bash
IS_BUY=true
AMOUNT=3000000
MIN_AMOUNT=1000000
SETTLE_CURRENCY="USD"
SETTLE_PRICE=1000000

secretcli tx compute execute "$P2P_CONTRACT" \
	'{
		"add_post": {
			"is_dealer_buy": '"$IS_BUY"',
			"amount": "'"$AMOUNT"'",
			"min_amount": "'"$MIN_AMOUNT"'",
			"settle_currency": "'"$SETTLE_CURRENCY"'",
			"settle_price": "'"$SETTLE_PRICE"'"
		}
	}' --from investor1 --fees 2500uscrt -y \
```

Post state is now `Open`.

### Dealer cancel a post

If post state is `Open` or `PendDealerDeposit`, dealer can cancel the entire post as no customer has entered to deal yet.

```bash
POST_ID=1
secretcli tx compute execute "$P2P_CONTRACT" \
	'{
		"cancel_post": {
			"cancel_post": "'"$POST_ID"'"
		}
	}' --from investor1 --fees 2500uscrt -y 
```

If a post is cancel, it will be deleted from the contract and no history will be stored.

### Customer enter a post and create a deal

```bash
POST_ID=1
AMOUNT=1000000
secretcli tx compute execute "$P2P_CONTRACT" \
	'{
		"enter_deal": {
			"post_id": "'"$POST_ID"'"
			"amount": "'"$AMOUNT"'"
		}
	}' --from localtest --fees 2500uscrt -y \
```

A few constraints would be checked before customer can enter a deal
- Customer has no other active deal at the moment
- Customer has his / her payment information setup
- Crypto amount that customer wants to trade should be more than the minimum amount set in the post
- Crypto amount that customer wants to trade should be low than the outstanding amount of the post

If dealer is selling crypto, deal state will become `pend_customer_bank_transfer`

If dealer is buying crypto (indicated by `is_dealer_buy` == `True`), deal state will become `pend_customer_deposit` and customer will need to deposit crypto after they enter a deal.

Notice that in the snip token `send` message this time, the key is `customer` while the nested dictionary references the deal_id, same as dealer deposit.

```bash
MESSAGE='{
	"customer": {
		"deal_id": "1"
	}
}'

BASE64_MSG=$(echo -n "$MESSAGE" | base64)
TOTAL_TOKEN=1000000

secretcli tx compute execute "$FINA_CONTRACT" \
	'{
		"send": {
			"recipient": "'"$P2P_CONTRACT"'",
			"amount": "'"$TOTAL_TOKEN"'",
			"msg": "'"$BASE64_MSG"'"
		}
	}' --from localtest --fees 2500uscrt -y
```

After customer deposit, deal state will become `pend_dealer_bank_transfer`.

After this stage, 
- `amount` from the post will be deduced by the deal `amount`, indicating a portion of the post is already being allocated to the deal.
- crypto must be deposited in the deal regardless of the direction of the deal.

### Deal expiry

Once customer enters the deal, a 6 hours expiry time is set. At each action, this timer will be reset. If time passes this timer, the counterpart can cancel the deal (See [below](#cancel-a-deal)). Or if the deal has reached the `signoff` stage and passed the expiry time, the counterpart can resolve the deal.

### Bank transfer

Up til this point, either customer/dealer will need to make a bank transfer to their counterpart (i.e. `pend_customer_bank_transfer` / `pend_dealer_bank_transfer`).
To know the bank transfer payment detail of their counterpart, they can run this query function, which restrict to only be viewable by the customer/dealer of the deal.

```bash
# QUERY_KEY is to verify sender. set it in query_auth contract.
QUERY_KEY="api_key_f+q9gV0pREYSfUJ5DuH/S/JfiH+a47pLziFrcEbHBl0="
ADDRESS="secret12xx45yytwu8sgl6p9qs8z2l99gkh73232q8n02"

secretcli q compute query "$P2P_CONTRACT" \
	'{
		"deal_detail": {
			"deal_id": "1",
			"key": "'"$QUERY_KEY"'",
			"address": "'"$ADDRESS"'"
		}
	}'
```

The output not only shows the deal detail, but also the payment info of the wire receiver.
```json
{
   "deal_detail":{
      "deal":{
         "deal_id":"1",
         "is_dealer_buy":false,
         "amount":"2000000",
         "settle_currency":"USD",
         "settle_price":"1000000",
         "dealer_deposit":true,
         "customer_deposit":false,
         "dealer":"secret16enhqv6ahewdy0kpuccvjj8rw88mzlrfna68m0",
         "customer":"secret12xx45yytwu8sgl6p9qs8z2l99gkh73232q8n02",
         "state":"pend_customer_bank_transfer",
         "resolver":null,
         "expiry":"1731850052"
      },
      "payment_info":{
         "method":"fps",
         "detail":"dealer number: 987654"
      }
   }
}
```

Customer / Dealer then makes the wire transfer offline and can update the deal once they complete the wire transfer by `confirm_bank_transfer` function.

```bash
DEAL_ID=1
secretcli tx compute execute "$P2P_CONTRACT" \
	'{
		"confirm_bank_transfer": {
			"deal_id": "'"$DEAL_ID"'"
		}
	}' --from localtest --fees 2500uscrt -y
```

After this, deal state becomes `pend_dealer_sign_off` or `pend_customer_sign_off`, depending on the direction.

### Resolve / Dispute the deal

Now, the counterpart can either run `resolve_deal` to resolve the deal, or `dispute_deal` to esculate the deal to moderator.

If a deal is resolved, the (deal amount - commission) will be the payout to receiver. Deal will be saved to `PAST_DEALS`.

```bash
DEAL_ID=1
secretcli tx compute execute "$P2P_CONTRACT" \
	'{
		"resolve_deal": {
			"deal_id": "'"$DEAL_ID"'"
		}
	}' --from investor1 --fees 2500uscrt -y 
```

```bash
DEAL_ID=1
secretcli tx compute execute "$P2P_CONTRACT" \
	'{
		"dispute_deal": {
			"deal_id": "'"$DEAL_ID"'"
		}
	}' --from localtest --fees 2500uscrt -y 
```

If a deal a dispute, mod can either run `resolve_deal` / `cancel_deal`. If a deal is cancel, crypto will be refund to the depositer of the deal.


### Cancel a deal

`cancel_deal` function is different from `cancel_post` and is available to use after customer entering a deal (meanwhile `cancel_post` can only be used by dealer before customer entering the deal).

`cancel_deal` will still push the deal to the `past_deals` so we could reference it later.

The table below shows who is able to `cancel_deal` at each deal state after the deal expiry time has passed.

| State                    | Who can cancel  | Crypto refund to |
|--------------------------|-----------------|-----------------|
| PendCustomerDeposit      | Dealer          | No crypto is refund |
| PendCustomerBankTransfer | Dealer          | Dealer |
| PendDealerBankTransfer   | Customer        | Customer |
| Dispute                  | Moderators      | If `is_dealer_buy`, refund to dealer, otherwlse customer |

Example execution function
```bash
DEAL_ID=1
secretcli tx compute execute "$P2P_CONTRACT" \
	'{
		"cancel_deal": {
			"deal_id": "'"$DEAL_ID"'"
		}
	}' --from localtest --fees 2500uscrt -y 
```

## Public Query function

All public query functions can be executed without parameter.

| Function | Description                                   |
|-------------------------|--------------------------------------------|
| config | Get the contract configuration |
| past_deals | Get the list of past resolved / cancelled deals |
| active_deals | Get the list of currently active deal |
| active_posts | Get the list of currently active post |
| revenue | Get the currenct commission revenue of the P2P contract |
| moderators | Get the list of moderators |

An example query
```bash
secretcli q compute query "$P2P_CONTRACT" \
	'{
		"active_deals": {}
	}'
```

## Admin Execution functions

1. Add Moderator (Control by governance if its available)

```bash
# Add moderator
MOD_ADDR="secret1zw8yc29flvrsp6qqe4ky446uahdnf6affc43x5"

secretcli tx compute execute "$P2P_CONTRACT" \
	'{
		"add_moderator": {
			"mod_addr": "'"$MOD_ADDR"'"
		}
	}' --from fina_ido --fees 2500uscrt -y
```

2. Remove Moderator (Control by governance if its available)

```bash
# Add moderator
MOD_ADDR="secret1zw8yc29flvrsp6qqe4ky446uahdnf6affc43x5"

secretcli tx compute execute "$P2P_CONTRACT" \
	'{
		"remove_moderator": {
			"mod_addr": "'"$MOD_ADDR"'"
		}
	}' --from fina_ido --fees 2500uscrt -y
```

3. Update deal token (Control by governance if its available) *PLEASE WITHDRAW COMMISSION BEFORE RUNNING THIS"

```bash
NEW_TOKEN_ADDR="secret1etrc3h558yesk25j5pu835f9s84wjvjzeed6e9"
NEW_TOKEN_HASH=$(secretcli query compute contract-hash "${NEW_TOKEN_ADDR}" | tail -c +3)

secretcli tx compute execute "$P2P_CONTRACT" \
	'{
		"update_deal_token": {
			"deal_token": {"address": "'"$NEW_TOKEN_ADDR"'","code_hash": "'"$DEAL_TOKEN_HASH"'"}
		}
	}' --from fina_ido --fees 2500uscrt -y 
```

4. Get Commission (Can only executed by admin)

```bash
secretcli tx compute execute "$P2P_CONTRACT" \
	'{
		"get_commission": {}
	}' --from fina_ido --fees 2500uscrt -y
```

5. Add Governance contract (Can only executed by admin)

```bash
GOV_ADDR=""
GOV_HASH=$(secretcli query compute contract-hash "${GOV_ADDR}" | tail -c +3)

secretcli tx compute execute "$P2P_CONTRACT" \
	'{
		"update_config": {
			"governance": {"address": "'"$GOV_ADDR"'","code_hash": "'"$GOV_HASH"'"}
		}
	}' --from fina_ido --fees 2500uscrt -y 
```