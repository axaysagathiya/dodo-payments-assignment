---
title: invoice-payment-design

---

# Overview

This service allows a business to:
* Register business
* Create customers
* Create invoices for customers
* Accept payments for invoices
* Receive webhook notifications when invoice status changes

The service stores data in PostgreSQL.

## 1. Data Model

### businesses

| Column |  Type  |
|:------:|:------:|
|   id   |  UUID  |
|  name  |  TEXT  |
| email  | CITEXT |

**Indexes:**
* Primary key on id

## api_keys
Stores API keys used by businesses.

|    Column     |  Type   |
|:-------------:|:-------:|
|      id       |  UUID   |
|  business_id  |  UUID   |
| hashed_secret |  TEXT   |
|    prefix     |  TEXT   |
|    active     | BOOLEAN |

### Indexes
* Primary key on id
* Index on business_id
* Index on prefix

### Reason
* API Key is "api_{prefix}_{secret}"
* secret stored as hash in database
* Prefix helps find the key quickly during authentication.

# customers

|   Column    |  Type  |
|:-----------:|:------:|
|     id      |  UUID  |
| business_id |  UUID  |
|    name     |  TEXT  |
|    email    | CITEXT |

### Indexes
* Primary key on id
* Index on business_id

### Reason
* Customers belong to a business

## invoices

|       Column       |  Type  |
|:------------------:|:------:|
|         id         |  UUID  |
|    business_id     |  UUID  |
|    customer_id     |  UUID  |
| total_amount_cents | BIGINT |
|       state        |  TEXT  |

### Indexes
* Primary key on id
* Index on (business_id, state)

### Reason
* Total amount is stored in cents
* Client cannot send invoice total_amount_cents. The server calculates them

## invoice_items

|     Column 1      | Column 3 |
|:-----------------:|:--------:|
|        id         |   UUID   |
|    invoice_id     |   UUID   |
|    description    |   TEXT   |
|     quantity      | INTEGER  |
| unit_amount_cents |  BIGINT  |

### Indexes
* Primary key on id
* Index on invoice_id

### Reason
Keeps invoice items separate from invoices

## payment_attempts

|    Column 1     | Column 3 |
|:---------------:|:--------:|
|       id        |   UUID   |
|   invoice_id    |   UUID   |
|  psp_reference  |   TEXT   |
|     status      |   TEXT   |
|  failure_code   |   TEXT   |
| idempotency_key |   TEXT   |

`psp_reference` stores the transaction identifier returned by the payment Processor.

### Indexes
* Primary key on id
* Unique index on (invoice_id, idempotency_key)

### Reason
* Prevents duplicate payment processing for the same request.

## TODO
* Add Schema for Webhooks
* What I would change at 100x scale
