
# Design
## Overview

The service stores data in PostgreSQL.

- So we would have an API to register the business, and the same API will create and return the API key to the business.
* Business will need an API key to:
    * create the customer
    * get all customer details, get customer detail by ID. 
    * create invoice for the customer
    * Get all invoices, get invoice filtered by Status, get invoice by id  
* For the payment, customer doesn't need any API key but they need to provide `idempotency key`. 

* The format of the API key is `api_prefix_secret`.
* implement an extractor from request parts To process the header and validate the API key because we need the business ID to create a customer and invoice. 

 The 60-second "freshness" window for 'processing' invoice  payment attempts prevents double-charging while allowing the system to recover from crashes

## 1. Data Model

### businesses

| Column |  Type  |
|:------:|:------:|
|   id   |  UUID  |
|  name  |  TEXT  |
| email  | CITEXT |

#### Indexes
* Primary key on id
* Unique index on email.

#### Reason
* email would be Unique to avoid creating duplicate businesses

### api_keys
Stores API keys used by businesses.

|    Column     |  Type   |
|:-------------:|:-------:|
|      id       |  UUID   |
|  business_id  |  UUID   |
| hashed_secret |  TEXT   |
|    prefix     |  TEXT   |
|    active     | BOOLEAN |

#### Indexes
* Primary key on id
* Index on business_id
* Unique Induction Prefix

#### Reason
* API Key is "api_{prefix}_{secret}"
* secret stored as hash in database
* Prefix helps find the key quickly during authentication.

### customers

|   Column    |  Type  |
|:-----------:|:------:|
|     id      |  UUID  |
| business_id |  UUID  |
|    name     |  TEXT  |
|    email    | CITEXT |

#### Indexes
* Primary key on id
* Index on business_id
* Unique index on (business, email).

#### Reason
* Customers belong to a business
* (business_id, email) would be unique to avoid creating duplicate customers for the business.

### invoices

|       Column       |  Type  |
|:------------------:|:------:|
|         id         |  UUID  |
|    business_id     |  UUID  |
|    customer_id     |  UUID  |
| total_amount_cents | BIGINT |
|       state        |  TEXT  |

state values:
For this assignment, we have Open and Paid. But in production, it could be draft, open, overdue, paid, uncollectible, cancelled.

#### Indexes
* Primary key on id
* Index on (business_id, state)

#### Reason
* Total amount is stored in cents


### invoice_items

|     Column     | Type |
|:-----------------:|:--------:|
|        id         |   UUID   |
|    invoice_id     |   UUID   |
|    description    |   TEXT   |
|     quantity      | INTEGER  |
| unit_amount_cents |  BIGINT  |

#### Indexes
* Primary key on id
* Index on invoice_id

#### Reason
Keeps invoice items separate from invoices

### payment_attempts

|     Column      |    Type     |
|:---------------:|:-----------:|
|       id        |    UUID     |
|   invoice_id    |    UUID     |
|  psp_reference  |    TEXT     |
|     status      |    TEXT     |
|  failure_code   |    TEXT     |
| idempotency_key |    TEXT     |
|   Created at    | TIMESTAMPTZ |
|   Updated it    | TIMESTAMPTZ |

`psp_reference` Stores the transaction identifier returned by the PSP for successful payments.

* Payment status could be:
- Unknown
- Failed
- Succeeded
- Processing

#### Indexes
* Primary key on id
* Unique index on (invoice_id, idempotency_key)

#### Reason
* Prevents duplicate payment processing for the same request.

### Webhook Endpoints.

|   Column    |   Type   |
|:-----------:|:--------:|
|     ID      |   UUID   |
| Business_id |   UUID   |
|     URL     |   Text   |
|   Secret    |   Text   |
|   Active    | Boolean. |

#### Indexes
* Primary key on ID. 
* Index on business_id

#### Reason 
Business register a webhook endpoint where events are delivered.

### webhook_events

|       Column        |  type   |
|:-------------------:|:-------:|
|         id,         |  UUID   |
| Webhook_endpoint_id |  UUID   |
|     Event_type      |  Text   |
|       Payload       | JasonB  |
|       Status        |  Text   |
|   Attempted_count   | integer |

#### Indexes
* Primary key on ID.
* Index on webhook_endpoint_id
* Index on status.

#### Reason 
* Webhook delivery may fail and require retries. 



---

### what you would change at 100x scale

- I would split very large tables such as invoices and payment attempts into smaller partitions based on creation date to keep queries fast. 
- I would add dedicated tables for webhook deliveries and retries so failed webhook notifications can be retried reliably. 
- I would also move old data to archive tables to keep day-to-day operations efficient. 
- If more APIs start using idempotency, I would create a separate table for idempotency records instead of storing them only in payment attempts.

## 2. Invoice State Machine

In the production system, states of invoice would be:
- draft
- open
- paid
- void

![invoice-state-machine](/images/Invoice%20state%20machine.jpeg)

**Reversible transitions:**
* draft -> Open, Open -> Draft
* Draft -> Void, Void -> Draft
* Open -> void, void -> open

**Terminal states** are Paid and Void

For the assignment, I will implement only open and paid.

**how invalid transitions are rejected**:
We only allow Payments for invoices with status open. 
* Open -> open  gets Rejected 
* paid -> paid gets Rejected


**Payment Attempt Outcomes**

Payment attempts are recorded separately from invoices.

![Payment-Attempt-Outcomes](/images/Payment%20attempt%20outcome.jpeg)

Payment states we would implement will be:
- processing
- succeed
- fail
- unknown

So when the customer sends the payment request, the invoice row gets locked and payment status gets set to processing.
* Now, it sends a request to PSP to process this transaction. 
* If the payment token is Token Success:
    * Payment status is equal to succeeded. 
    * Invoice status is equal to paid.
* If the payment token is `insufficient funds` or `card declined`:
    * Payment status = failed
    * invoice status = open. 
* If the token is a `timeout` or `network error`:
    * Payment status = unknown(Because we are not sure that the payment will succeed or fail)
    * Invoice status = open.
* If the PSP service is not reachable or customer Is unable to receive the response from PSP, the customer will get an error response. At this time: 
    * payment status = processing
    * the invoice status is open.

For processing an unknown payment status, in production the customer can call the API to see the payment status. If the payment is successful, they would get the transaction ID, but in  assignment We don't have actual payment logic in PSP. So it would not be possible to recall the API, and get the updated status. 

the payments in the 'processing' state do not have any mechanism to get failed or succeeded, so we can have a time window. For example, if the payment is in the processing state for 60 seconds, we can consider it failed, allow the payment again.

## 3. Payment Correctness & Failure Modes

### (a) Two clients call POST /invoices/{id}/pay for the same invoice at the same instant. What is the outcome? What mechanism guarantees this?

Before processing payment, the service locks the invoice row using a database row lock.Only one request can continue. The customer would get informed that payment of this invoice is already in process. 

The mechanism name is row-level lock And the reason behind choosing this is it's simple, very easy to reason about, and PostgreSQL already provides this feature. 

### (b) The mock PSP times out (tok_timeout, 30 s). What does your endpoint return? What state is the invoice or payment_attempt left in? How does the caller find out the eventual result?

* Invoice state stays open.
* Payment attempt becomes `unknown` status, as we cannot be sure about whether the payment succeeded or failed.
* return 504 Gateway Timeout
* In the production system, this unknown status would eventually be succeeded or failed, but as we do not have that mechanism, we would not have that mechanism. We will allow or retry payment.

### ( C ) PSP succeeded but service crashed before saving result

- In that case, the client can retry using the same item potency key.
- Service checks the existing payment first, and if the payment has already succeeded, the Projection ID will be returned. Otherwise, amount will be deducted.
- No duplicate charge occurs.

### (D) Same idempotency key with different request body

* In that case, the request would get rejected with 409 Conflict. 

### (E) Paying already paid invoice 
The request would get rejected with 409 Conflict, as a paid invoice is a terminal state.


## Webhook Design

Webhook requests can be signed using SHA256 and a secret shared with each business. The signature is generated from the request payload and a timestamp, allowing businesses to verify that the webhook came from this service and reject old requests to prevent replay attacks.

Webhook delivery should happen in the background instead of during the API request. When an invoice status changes, the event is first saved, and the API response is returned immediately. A separate worker can then deliver the webhook. This prevents slow or unavailable business endpoints from affecting the main invoice flow.

If webhook delivery fails, the service can retry after increasing intervals, for example after 1 minute, 5 minutes, 30 minutes, 2 hours, and 24 hours. After a fixed number of attempts (for example, 5), the event can be marked as failed and no further automatic retries are performed.

To recover from missed webhooks, businesses can use an API endpoint to fetch past events and reconcile their systems if needed.

If I implement this, invoice.paid and invoice.payment_failed are the most valuable webhook events because they represent state changes that happen after the original API request. I would not add a webhook for invoice.created if there is no other service, at least not mentioned in this assignment.

## API Key model. 
* So the business can register, and at the same time, an API key will get generated.
* I would generate the API in this format: **API_ prefix_secret**. (8 bytes of prefix and 32 Bites of Secret )
* **Storage:**
    * Store prefix in the database for easy search.
    * Store hash of the secret for the security.
* **Usage:** We can send this API key in the authorization header. 
* **Rotation**: So in production, we can send email to the business that would create a new API and  the old API key.(But for the assignment's purpose, I would not implement this.)
* In the database, in the API key table, we can have the `active` column in which we can set a false.Also, would not implement the API for this in the assignment)
* **If leaked,** only one business was affected because the key belongs to a single business.

## What I cut and why 
* In the invoice payment, I would implement only for status paid and unpaid(Open). As both are most important for the payment life cycle
* I would not implement partial payments for the invoice to avoid invoice state complexity.
* Also would not implement email notifications and mail to generate the API, as it is out of the scope of this assignment.
* So when the PSP returns the unknown status in production, should re-try the payment, but I would not do that for this assignment. 

## Production Readiness Gap

I would 
* In many database tables, I have not used the created and updated fields that would be required in a production system. 
* add more invoice state and payment states.
* add better monitoring metrics, logs, and alerts.
* Rate limiting to prevent abuse and accidental overload
* Audit logs to track important actions such as payment and API key changes
* mechanism to verify the business when creating the API key, modifying the API key, updating the API key, or revoking the API key.
* In production, we can have due date for the invoice