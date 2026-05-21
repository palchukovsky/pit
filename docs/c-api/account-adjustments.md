# Account Adjustments

<!-- markdownlint-disable MD013 MD024 -->

[Back to index](index.md)

## `OpenPitParamAdjustmentAmount`

One amount component inside an account adjustment.

The numeric value is interpreted according to `kind`:

- `Delta` means "change current state by this signed amount";
- `Absolute` means "set current state to this signed amount".

```c
typedef struct OpenPitParamAdjustmentAmount {
    OpenPitParamPositionSize value;
    OpenPitParamAdjustmentAmountKind kind;
} OpenPitParamAdjustmentAmount;
```

## `OpenPitAccountAdjustmentBalanceOperation`

Balance-operation payload for account adjustment.

```c
typedef struct OpenPitAccountAdjustmentBalanceOperation {
    OpenPitStringView asset;
    OpenPitParamPriceOptional average_entry_price;
} OpenPitAccountAdjustmentBalanceOperation;
```

## `OpenPitAccountAdjustmentPositionOperation`

Position-operation payload for account adjustment.

```c
typedef struct OpenPitAccountAdjustmentPositionOperation {
    OpenPitInstrument instrument;
    OpenPitStringView collateral_asset;
    OpenPitParamPriceOptional average_entry_price;
    OpenPitParamLeverage leverage;
    OpenPitParamPositionMode mode;
} OpenPitAccountAdjustmentPositionOperation;
```

## `OpenPitAccountAdjustmentBalanceOperationOptional`

```c
typedef struct OpenPitAccountAdjustmentBalanceOperationOptional {
    OpenPitAccountAdjustmentBalanceOperation value;
    bool is_set;
} OpenPitAccountAdjustmentBalanceOperationOptional;
```

## `OpenPitAccountAdjustmentPositionOperationOptional`

```c
typedef struct OpenPitAccountAdjustmentPositionOperationOptional {
    OpenPitAccountAdjustmentPositionOperation value;
    bool is_set;
} OpenPitAccountAdjustmentPositionOperationOptional;
```

## `OpenPitAccountAdjustmentAmount`

Optional amount-change group for account adjustment.

The group is absent when every field is absent.

```c
typedef struct OpenPitAccountAdjustmentAmount {
    OpenPitParamAdjustmentAmount balance;
    OpenPitParamAdjustmentAmount held;
    OpenPitParamAdjustmentAmount incoming;
} OpenPitAccountAdjustmentAmount;
```

## `OpenPitAccountAdjustmentBounds`

Optional bounds group for account adjustment.

The group is absent when every bound is absent.

```c
typedef struct OpenPitAccountAdjustmentBounds {
    OpenPitParamPositionSizeOptional balance_upper;
    OpenPitParamPositionSizeOptional balance_lower;
    OpenPitParamPositionSizeOptional held_upper;
    OpenPitParamPositionSizeOptional held_lower;
    OpenPitParamPositionSizeOptional incoming_upper;
    OpenPitParamPositionSizeOptional incoming_lower;
} OpenPitAccountAdjustmentBounds;
```

## `OpenPitAccountAdjustment`

Full caller-owned account-adjustment payload.

```c
typedef struct OpenPitAccountAdjustment {
    OpenPitAccountAdjustmentBalanceOperationOptional balance_operation;
    OpenPitAccountAdjustmentPositionOperationOptional position_operation;
    OpenPitAccountAdjustmentAmountOptional amount;
    OpenPitAccountAdjustmentBoundsOptional bounds;
    void * user_data;
} OpenPitAccountAdjustment;
```

## `OpenPitAccountAdjustmentApplyStatus`

Result of `openpit_engine_apply_account_adjustment`.

```c
typedef uint8_t OpenPitAccountAdjustmentApplyStatus;
/**
 * The call failed before the batch could be evaluated.
 */
#define OpenPitAccountAdjustmentApplyStatus_Error \
    ((OpenPitAccountAdjustmentApplyStatus) 0)
/**
 * The batch was accepted and applied.
 */
#define OpenPitAccountAdjustmentApplyStatus_Applied \
    ((OpenPitAccountAdjustmentApplyStatus) 1)
/**
 * The batch was evaluated and rejected by policy or validation logic.
 */
#define OpenPitAccountAdjustmentApplyStatus_Rejected \
    ((OpenPitAccountAdjustmentApplyStatus) 2)
```

## `OpenPitAccountAdjustmentAmountOptional`

```c
typedef struct OpenPitAccountAdjustmentAmountOptional {
    OpenPitAccountAdjustmentAmount value;
    bool is_set;
} OpenPitAccountAdjustmentAmountOptional;
```

## `OpenPitAccountAdjustmentBoundsOptional`

```c
typedef struct OpenPitAccountAdjustmentBoundsOptional {
    OpenPitAccountAdjustmentBounds value;
    bool is_set;
} OpenPitAccountAdjustmentBoundsOptional;
```
