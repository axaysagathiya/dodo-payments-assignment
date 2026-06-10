# cURL Commands

You can replace the variables like `{{api_key}}`, `{{customer_id}}`, and `{{invoice_id}}` with your actual values before executing them in your terminal.

First, register a business to get an API Key:

### 1. Register a business
```bash
curl -X POST http://localhost:8080/business \
  -H "Content-Type: application/json" \
  -d '{"name": "star-cafe2", "email": "info@starcafe2.in"}'
```
**Save the `api_key` from the response**

### 2. Create a customer for a business (without authentication)
```bash
curl -X POST http://localhost:8080/customer \
  -H "Content-Type: application/json" \
  -d '{"name": "axay", "email": "axay@gmail.com"}'
```

### 3. Create a customer for a business (with authentication)
```bash
curl -X POST http://localhost:8080/customer \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer {{api_key}}" \
  -d '{"name": "axay", "email": "axay@gmail.com"}'
```

### 4. Fetch customers for a business
```bash
curl -X GET http://localhost:8080/customers \
  -H "Authorization: Bearer {{api_key}}"
```

### 5. Fetch a specific customer for a business
```bash
curl -X GET http://localhost:8080/customers/{{customer_id}} \
  -H "Authorization: Bearer {{api_key}}"
```

### 6. Create an invoice for a customer
```bash
curl -X POST http://localhost:8080/invoice \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer {{api_key}}" \
  -d '{"customer_id": "{{customer_id}}","items": [{"description": "AC2","quantity": 3,"unit_amount_cents": 300},{"description": "TV2","quantity": 2,"unit_amount_cents": 600}]}'
```

### 7. Fetch a specific invoice
```bash
curl -X GET http://localhost:8080/invoice/{{invoice_id}} \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer {{api_key}}"
```

### 8. Fetch invoices with a status filter
```bash
curl -X GET "http://localhost:8080/invoices?status=open" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer {{api_key}}"
```

### 9. Fetch all invoices for a business
```bash
curl -X GET http://localhost:8080/invoices \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer {{api_key}}"
```

### 10. Pay an invoice (tok_network_error)
```bash
curl -X POST http://localhost:8080/invoices/{{invoice_id}}/pay \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer {{api_key}}" \
  -d '{"card_token": "tok_network_error","idempotency_key": "unique_key_126"}'
```

### 11. Pay an invoice (tok_timeout)
This will return a 504/Processing status after 20 seconds

```bash
curl -X POST http://localhost:8080/invoices/{{invoice_id}}/pay \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer {{api_key}}" \
  -d '{"card_token": "tok_timeout","idempotency_key": "unique_key_128"}'
```
- Wait 60 seconds and retry with a new key to see staleness recovery
- or within 60 seconds, try calling any api to pay again for the same invoice, you gonna get A response saying that payment of this invoice is already in process 

### 12. Pay an invoice (tok_card_declined)
```bash
curl -X POST http://localhost:8080/invoices/{{invoice_id}}/pay \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer {{api_key}}" \
  -d '{"card_token": "tok_card_declined","idempotency_key": "unique_key_127"}'
```

### 13. Pay an invoice (tok_success)
```bash
curl -X POST http://localhost:8080/invoices/{{invoice_id}}/pay \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer {{api_key}}" \
  -d '{"card_token": "tok_success","idempotency_key": "unique_key_125"}'
```