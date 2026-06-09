
CREATE EXTENSION IF NOT EXISTS citext;

CREATE TABLE businesses (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    email CITEXT NOT NULL UNIQUE
);

CREATE TABLE api_keys (
    id UUID PRIMARY KEY,
    business_id UUID NOT NULL REFERENCES businesses(id),
    hashed_secret TEXT NOT NULL,
    prefix TEXT NOT NULL UNIQUE,
    active BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE INDEX idx_api_keys_business_id ON api_keys(business_id);
CREATE INDEX idx_api_keys_prefix ON api_keys(prefix);

CREATE TABLE customers (
    id UUID PRIMARY KEY,
    business_id UUID NOT NULL REFERENCES businesses(id),
    name TEXT NOT NULL,
    email CITEXT NOT NULL,

    CONSTRAINT uq_customer_business_email UNIQUE (business_id, email)
);

CREATE INDEX idx_customers_business_id ON customers(business_id);

CREATE TABLE invoices (
    id UUID PRIMARY KEY,
    business_id UUID NOT NULL REFERENCES businesses(id),
    customer_id UUID NOT NULL REFERENCES customers(id),
    total_amount_cents BIGINT NOT NULL,
    state TEXT NOT NULL
);

CREATE INDEX idx_invoices_business_state ON invoices(business_id, state);

CREATE TABLE invoice_items (
    id UUID PRIMARY KEY,
    invoice_id UUID NOT NULL REFERENCES invoices(id),
    description TEXT NOT NULL,
    quantity INTEGER NOT NULL,
    unit_amount_cents BIGINT NOT NULL
);

CREATE INDEX idx_invoice_items_invoice_id ON invoice_items(invoice_id);

CREATE TABLE payment_attempts (
    id UUID PRIMARY KEY,
    invoice_id UUID NOT NULL REFERENCES invoices(id),
    psp_reference TEXT,
    status TEXT NOT NULL,
    failure_code TEXT,
    idempotency_key TEXT NOT NULL,
    request_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_payment_attempts_idempotency ON payment_attempts(invoice_id, idempotency_key);
