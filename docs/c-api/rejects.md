# Rejects

<!-- markdownlint-disable MD013 MD024 -->

[Back to index](index.md)

## `OpenPitRejectScope`

Broad area to which a reject applies.

Valid values: `Order` (1), `Account` (2). Zero is not a valid scope value; the
caller must always set this field explicitly.

```c
typedef uint8_t OpenPitRejectScope;
/**
 * The reject applies to one order or order-like request.
 */
#define OpenPitRejectScope_Order ((OpenPitRejectScope) 1)
/**
 * The reject applies to account state rather than to one order only.
 */
#define OpenPitRejectScope_Account ((OpenPitRejectScope) 2)
```

## `OpenPitRejectCode`

Stable classification code for a reject.

Read this first when you need machine-readable handling. The textual fields in
[`OpenPitReject`] provide operator-facing explanation and extra context.

Valid codes are `1..=39` and `255` (`Other`). Unknown incoming codes are mapped
to `Other` (`255`).

```c
typedef uint16_t OpenPitRejectCode;
/**
 * A required field is absent.
 */
#define OpenPitRejectCode_MissingRequiredField ((OpenPitRejectCode) 1)
/**
 * A field cannot be parsed from the supplied wire value.
 */
#define OpenPitRejectCode_InvalidFieldFormat ((OpenPitRejectCode) 2)
/**
 * A field is syntactically valid but semantically unacceptable.
 */
#define OpenPitRejectCode_InvalidFieldValue ((OpenPitRejectCode) 3)
/**
 * The requested order type is not supported.
 */
#define OpenPitRejectCode_UnsupportedOrderType ((OpenPitRejectCode) 4)
/**
 * The requested time-in-force is not supported.
 */
#define OpenPitRejectCode_UnsupportedTimeInForce ((OpenPitRejectCode) 5)
/**
 * Another order attribute is unsupported.
 */
#define OpenPitRejectCode_UnsupportedOrderAttribute ((OpenPitRejectCode) 6)
/**
 * The client order identifier duplicates an active order.
 */
#define OpenPitRejectCode_DuplicateClientOrderId ((OpenPitRejectCode) 7)
/**
 * The order arrived after the allowed entry deadline.
 */
#define OpenPitRejectCode_TooLateToEnter ((OpenPitRejectCode) 8)
/**
 * Trading is closed for the relevant venue or session.
 */
#define OpenPitRejectCode_ExchangeClosed ((OpenPitRejectCode) 9)
/**
 * The instrument cannot be resolved.
 */
#define OpenPitRejectCode_UnknownInstrument ((OpenPitRejectCode) 10)
/**
 * The account cannot be resolved.
 */
#define OpenPitRejectCode_UnknownAccount ((OpenPitRejectCode) 11)
/**
 * The venue cannot be resolved.
 */
#define OpenPitRejectCode_UnknownVenue ((OpenPitRejectCode) 12)
/**
 * The clearing account cannot be resolved.
 */
#define OpenPitRejectCode_UnknownClearingAccount ((OpenPitRejectCode) 13)
/**
 * The collateral asset cannot be resolved.
 */
#define OpenPitRejectCode_UnknownCollateralAsset ((OpenPitRejectCode) 14)
/**
 * Available balance is insufficient.
 */
#define OpenPitRejectCode_InsufficientFunds ((OpenPitRejectCode) 15)
/**
 * Available margin is insufficient.
 */
#define OpenPitRejectCode_InsufficientMargin ((OpenPitRejectCode) 16)
/**
 * Available position is insufficient.
 */
#define OpenPitRejectCode_InsufficientPosition ((OpenPitRejectCode) 17)
/**
 * A credit limit was exceeded.
 */
#define OpenPitRejectCode_CreditLimitExceeded ((OpenPitRejectCode) 18)
/**
 * A risk limit was exceeded.
 */
#define OpenPitRejectCode_RiskLimitExceeded ((OpenPitRejectCode) 19)
/**
 * The order exceeds a generic configured limit.
 */
#define OpenPitRejectCode_OrderExceedsLimit ((OpenPitRejectCode) 20)
/**
 * The order quantity exceeds a configured limit.
 */
#define OpenPitRejectCode_OrderQtyExceedsLimit ((OpenPitRejectCode) 21)
/**
 * The order notional exceeds a configured limit.
 */
#define OpenPitRejectCode_OrderNotionalExceedsLimit ((OpenPitRejectCode) 22)
/**
 * The resulting position exceeds a configured limit.
 */
#define OpenPitRejectCode_PositionLimitExceeded ((OpenPitRejectCode) 23)
/**
 * Concentration constraints were violated.
 */
#define OpenPitRejectCode_ConcentrationLimitExceeded ((OpenPitRejectCode) 24)
/**
 * Leverage constraints were violated.
 */
#define OpenPitRejectCode_LeverageLimitExceeded ((OpenPitRejectCode) 25)
/**
 * The request rate exceeded a configured limit.
 */
#define OpenPitRejectCode_RateLimitExceeded ((OpenPitRejectCode) 26)
/**
 * A loss barrier has blocked further risk-taking.
 */
#define OpenPitRejectCode_PnlKillSwitchTriggered ((OpenPitRejectCode) 27)
/**
 * The account is blocked.
 */
#define OpenPitRejectCode_AccountBlocked ((OpenPitRejectCode) 28)
/**
 * The account is not authorized for this action.
 */
#define OpenPitRejectCode_AccountNotAuthorized ((OpenPitRejectCode) 29)
/**
 * A compliance restriction blocked the action.
 */
#define OpenPitRejectCode_ComplianceRestriction ((OpenPitRejectCode) 30)
/**
 * The instrument is restricted.
 */
#define OpenPitRejectCode_InstrumentRestricted ((OpenPitRejectCode) 31)
/**
 * A jurisdiction restriction blocked the action.
 */
#define OpenPitRejectCode_JurisdictionRestriction ((OpenPitRejectCode) 32)
/**
 * The action would violate wash-trade prevention.
 */
#define OpenPitRejectCode_WashTradePrevention ((OpenPitRejectCode) 33)
/**
 * The action would violate self-match prevention.
 */
#define OpenPitRejectCode_SelfMatchPrevention ((OpenPitRejectCode) 34)
/**
 * Short-sale restriction blocked the action.
 */
#define OpenPitRejectCode_ShortSaleRestriction ((OpenPitRejectCode) 35)
/**
 * Required risk configuration is missing.
 */
#define OpenPitRejectCode_RiskConfigurationMissing ((OpenPitRejectCode) 36)
/**
 * Required reference data is unavailable.
 */
#define OpenPitRejectCode_ReferenceDataUnavailable ((OpenPitRejectCode) 37)
/**
 * The system could not compute an order value needed for validation.
 */
#define OpenPitRejectCode_OrderValueCalculationFailed ((OpenPitRejectCode) 38)
/**
 * A required service or subsystem is unavailable.
 */
#define OpenPitRejectCode_SystemUnavailable ((OpenPitRejectCode) 39)
/**
 * Reserved discriminant for caller-defined reject classes.
 *
 * Use together with `Reject::with_user_data` to attach a caller-defined
 * payload that the receiving code can decode. The SDK does not interpret this
 * code beyond mapping it to FFI value 254.
 */
#define OpenPitRejectCode_Custom ((OpenPitRejectCode) 254)
/**
 * A catch-all code for rejects that do not fit a more specific class.
 */
#define OpenPitRejectCode_Other ((OpenPitRejectCode) 255)
```

## `OpenPitReject`

Single rejection record returned by checks.

```c
typedef struct OpenPitReject {
    OpenPitStringView policy;
    OpenPitStringView reason;
    OpenPitStringView details;
    void * user_data;
    OpenPitRejectCode code;
    OpenPitRejectScope scope;
} OpenPitReject;
```

## `OpenPitRejectList`

Caller-owned list of rejects.

```c
typedef struct OpenPitRejectList OpenPitRejectList;
```

## `openpit_create_reject_list`

Creates a caller-owned reject list with preallocated capacity.

`reserve` is the requested number of elements to preallocate.

Contract:

- returns a new caller-owned list;
- release it with `openpit_destroy_reject_list`;
- this function always succeeds.

```c
OpenPitRejectList * openpit_create_reject_list(
    size_t reserve
);
```

## `openpit_destroy_reject_list`

Releases a caller-owned reject list.

Contract:

- passing null is allowed;
- this function always succeeds.

```c
void openpit_destroy_reject_list(
    OpenPitRejectList * rejects
);
```

## `openpit_reject_list_push`

Appends one reject to the list by copying its payload.

Contract:

- `list` must be a valid non-null pointer;
- string views in `reject` are copied before this function returns;
- this function never fails;
- violating the pointer contract aborts the call.

```c
void openpit_reject_list_push(
    OpenPitRejectList * list,
    OpenPitReject reject
);
```

## `openpit_reject_list_len`

Returns the number of rejects in the list.

Contract:

- `list` must be a valid non-null pointer;
- this function never fails;
- violating the pointer contract aborts the call.

```c
size_t openpit_reject_list_len(
    const OpenPitRejectList * list
);
```

## `openpit_reject_list_get`

Copies a non-owning reject view at `index` into `out_reject`.

The copied view borrows string memory from `list`.

Contract:

- `list` must be a valid non-null pointer;
- `out_reject` must be a valid non-null pointer;
- returns `true` when a value exists and was copied;
- returns `false` when `index` is out of bounds and does not write
  `out_reject`;
- the copied view remains valid while `list` is alive and unchanged;
- this function never fails;
- violating the pointer contract aborts the call.

```c
bool openpit_reject_list_get(
    const OpenPitRejectList * list,
    size_t index,
    OpenPitReject * out_reject
);
```
