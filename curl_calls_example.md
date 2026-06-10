# Api call Examples

### POST request to register a business
```
curl --request POST \
  --url http://localhost:8080/business \
  --header 'content-type: application/json' \
  --header 'user-agent: vscode-restclient' \
  --data '{"name": "star-cafe2","email": "info@starcafe2.in"}'
```

```
HTTP/1.1 200 OK
content-type: application/json
content-length: 112
connection: close
date: Tue, 09 Jun 2026 18:22:22 GMT

{
  "business_id": "b8da6882-596a-42ab-8a15-d32747daf5bf",
  "api_key": "API_HUsECVbA_71KKj04DDoogYK09XHdSBqM4EWd9WyjL"
}
```

### POST request to create a customer for a business without authentication
```
curl --request POST \
  --url http://localhost:8080/customer \
  --header 'content-type: application/json' \
  --header 'user-agent: vscode-restclient' \
  --data '{"name": "axay","email": "axay@gmail.com"}'
```

```
HTTP/1.1 401 Unauthorized
content-type: application/json
content-length: 79
connection: close
date: Tue, 09 Jun 2026 18:26:06 GMT

{
  "code": "UNAUTHORIZED",
  "message": "Missing Authorization header",
  "details": null
}
```

### POST request to create a customer for a business with authentication
```
curl --request POST \
  --url http://localhost:8080/customer \
  --header 'authorization: Bearer API_HUsECVbA_71KKj04DDoogYK09XHdSBqM4EWd9WyjL' \
  --header 'content-type: application/json' \
  --header 'user-agent: vscode-restclient' \
  --data '{"name": "axay","email": "axay@gmail.com"}'
```

```
HTTP/1.1 200 OK
content-type: application/json
content-length: 54
connection: close
date: Tue, 09 Jun 2026 18:28:41 GMT

{
  "customer_id": "e0c8f203-d59d-4487-9e14-1c1001734df2"
}
```

### POST request to create an invoice for a customer
```
curl --request POST \
  --url http://localhost:8080/invoice \
  --header 'authorization: Bearer API_HUsECVbA_71KKj04DDoogYK09XHdSBqM4EWd9WyjL' \
  --header 'content-type: application/json' \
  --header 'user-agent: vscode-restclient' \
  --data '{"customer_id": "e0c8f203-d59d-4487-9e14-1c1001734df2","items": [{"description": "AC2","quantity": 3,"unit_amount_cents": 300},{"description": "TV2","quantity": 2,"unit_amount_cents": 600}]}'
```

```
HTTP/1.1 200 OK
content-type: application/json
content-length: 53
connection: close
date: Tue, 09 Jun 2026 18:37:35 GMT

{
  "invoice_id": "99cf996b-0083-42f6-a419-dfd7a70e542f"
}
```

### POST request to pay for an invoice (Public API)

#### tok_timeout
```
curl --request POST \
  --url http://localhost:8080/invoices/99cf996b-0083-42f6-a419-dfd7a70e542f/pay \
  --header 'authorization: Bearer API_HUsECVbA_71KKj04DDoogYK09XHdSBqM4EWd9WyjL' \
  --header 'content-type: application/json' \
  --header 'user-agent: vscode-restclient' \
  --data '{"card_token": "tok_timeout","idempotency_key": "unique_key_128"}'
```

```
HTTP/1.1 504 Gateway Timeout
content-type: application/json
content-length: 100
connection: close
date: Tue, 09 Jun 2026 18:33:53 GMT

{
  "code": "PAYMENT_STATUS_UNKNOWN",
  "message": "Payment result: unknown (code: timeout)",
  "details": null
}
```

#### tok_card_declined, used idempotency_key
```
curl --request POST \
  --url http://localhost:8080/invoices/99cf996b-0083-42f6-a419-dfd7a70e542f/pay \
  --header 'authorization: Bearer API_HUsECVbA_71KKj04DDoogYK09XHdSBqM4EWd9WyjL' \
  --header 'content-type: application/json' \
  --header 'user-agent: vscode-restclient' \
  --data '{"card_token": "tok_card_declined","idempotency_key": "unique_key_128"}'
```

```
HTTP/1.1 409 Conflict
content-type: application/json
content-length: 100
connection: close
date: Tue, 09 Jun 2026 18:48:04 GMT

{
  "code": "CONFLICT",
  "message": "Idempotency key reused with different request payload",
  "details": null
}
```

#### tok_card_declined, new idempotency_key
```
curl --request POST \
  --url http://localhost:8080/invoices/99cf996b-0083-42f6-a419-dfd7a70e542f/pay \
  --header 'authorization: Bearer API_HUsECVbA_71KKj04DDoogYK09XHdSBqM4EWd9WyjL' \
  --header 'content-type: application/json' \
  --header 'user-agent: vscode-restclient' \
  --data '{"card_token": "tok_card_declined","idempotency_key": "unique_key_127"}'
```

```
HTTP/1.1 402 Payment Required
content-type: application/json
content-length: 97
connection: close
date: Tue, 09 Jun 2026 18:50:01 GMT

{
  "code": "PAYMENT_FAILED",
  "message": "Payment result: failed (code: card_declined)",
  "details": null
}
```

### tok_network_error
```
curl --request POST \
  --url http://localhost:8080/invoices/99cf996b-0083-42f6-a419-dfd7a70e542f/pay \
  --header 'authorization: Bearer API_HUsECVbA_71KKj04DDoogYK09XHdSBqM4EWd9WyjL' \
  --header 'content-type: application/json' \
  --header 'user-agent: vscode-restclient' \
  --data '{"card_token": "tok_network_error","idempotency_key": "unique_key_126"}'
```

```
HTTP/1.1 500 Internal Server Error
content-type: application/json
content-length: 106
connection: close
date: Tue, 09 Jun 2026 18:52:08 GMT

{
  "code": "PAYMENT_STATUS_UNKNOWN",
  "message": "Payment result: unknown (code: network_error)",
  "details": null
}
```

### tok_success
```
curl --request POST \
  --url http://localhost:8080/invoices/99cf996b-0083-42f6-a419-dfd7a70e542f/pay \
  --header '"card_token": "tok_success",' \
  --header '"idempotency_key": "unique_key_125"' \
  --header 'authorization: Bearer API_HUsECVbA_71KKj04DDoogYK09XHdSBqM4EWd9WyjL' \
  --header 'content-type: application/json' \
  --header 'user-agent: vscode-restclient'
```

```
HTTP/1.1 200 OK
content-type: application/json
content-length: 153
connection: close
date: Tue, 09 Jun 2026 18:56:15 GMT

{
  "attempt_id": "51bb588b-4b5f-4dec-83ba-c0605e2ef0e5",
  "status": "succeeded",
  "psp_reference": "txn_164fef25-c371-4c88-8090-0093c969059f",
  "failure_code": null
}
```
 
### try registering business when database is not connected
(Could be any API)

Response:
```
HTTP/1.1 500 Internal Server Error
content-type: application/json
content-length: 174
connection: close
date: Wed, 10 Jun 2026 06:00:43 GMT

{
  "code": "DATABASE_ERROR",
  "message": "Failed to create business",
  "details": "error communicating with database: failed to lookup address information: Name or service not known"
}
```

### Try calling pay api when mock_psp is unreachable

```
HTTP/1.1 503 Service Unavailable
content-type: application/json
content-length: 108
connection: close
date: Wed, 10 Jun 2026 06:24:25 GMT

{
  "code": "PAYMENT_PROCESSING",
  "message": "Payment result: processing (code: connection_error)",
  "details": null
}
```