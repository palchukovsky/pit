# Orders

<!-- markdownlint-disable MD013 MD024 -->

[Back to index](index.md)

## `OpenPitOrderOperation`

Optional operation group for an order.

The group is absent when all fields are absent.

```c
typedef struct OpenPitOrderOperation {
    OpenPitParamTradeAmount trade_amount;
    OpenPitInstrument instrument;
    OpenPitParamPriceOptional price;
    OpenPitParamAccountIdOptional account_id;
    OpenPitParamSide side;
} OpenPitOrderOperation;
```

## `OpenPitOrderPosition`

Optional position-management group for an order.

The group is absent when every field is `NotSet`.

```c
typedef struct OpenPitOrderPosition {
    OpenPitParamPositionSide position_side;
    OpenPitTriBool reduce_only;
    OpenPitTriBool close_position;
} OpenPitOrderPosition;
```

## `OpenPitOrderMargin`

Optional margin group for an order.

The group is absent when every field is `NotSet`.

```c
typedef struct OpenPitOrderMargin {
    OpenPitStringView collateral_asset;
    OpenPitTriBool auto_borrow;
    OpenPitParamLeverage leverage;
} OpenPitOrderMargin;
```

## `OpenPitOrder`

Full caller-owned order payload.

```c
typedef struct OpenPitOrder {
    OpenPitOrderOperationOptional operation;
    OpenPitOrderMarginOptional margin;
    OpenPitOrderPositionOptional position;
    void * user_data;
} OpenPitOrder;
```

## `OpenPitOrderOperationOptional`

```c
typedef struct OpenPitOrderOperationOptional {
    OpenPitOrderOperation value;
    bool is_set;
} OpenPitOrderOperationOptional;
```

## `OpenPitOrderMarginOptional`

```c
typedef struct OpenPitOrderMarginOptional {
    OpenPitOrderMargin value;
    bool is_set;
} OpenPitOrderMarginOptional;
```

## `OpenPitOrderPositionOptional`

```c
typedef struct OpenPitOrderPositionOptional {
    OpenPitOrderPosition value;
    bool is_set;
} OpenPitOrderPositionOptional;
```
